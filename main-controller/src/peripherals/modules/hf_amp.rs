use embassy_time::{Duration, Instant, Timer};

use crate::app::cordic_math::{with_cordic, CordicMutex};
use crate::app::types::{Mode, PaTemperatures};
use crate::control_board::events::{
    PdContract, PowerTelemetry, PA_CURRENT_READING, PA_CURRENT_REQUEST, PA_FAST_MODE,
    RAIL_50V_READY,
};
use crate::i2c_map::I2cAddress;
use crate::peripherals::types::{PeripherialI2c, PeripherialI2cMutex};
use common::drivers::ads1115::{ADS1115, ADS1115Config};
use common::drivers::mcp4725::MCP4725;

const FINAL_TARGET_IDQ_MA: i32 = 150;
const FINAL_TOLERANCE_MA: i32 = 20;
const DRIVER_TARGET_IDQ_MA: i32 = 100;
const DRIVER_TOLERANCE_MA: i32 = 20;
const CALIBRATION_STALE_MS: u64 = 60_000;
const DAC_COARSE_STEP: u16 = 8;
const DAC_FINE_STEP: u16 = 2;

const THERMAL_DERATING_START_C: i16 = 50;
const THERMAL_AD8367_ZERO_C: i16 = 70;
const THERMAL_EMERGENCY_C: i16 = 80;

const NTC_R0: f32 = 10000.0;
const NTC_T0: f32 = 298.15;
const NTC_B: f32 = 3455.0;
const PULL_UP_R: f32 = 10000.0;
const VCC: f32 = 3.3;
const ADC_FSR: f32 = 4.096;
const ADC_MAX: f32 = 32768.0;

const CALIBRATION_TEMP_DRIFT_C: i16 = 15;

const RAIL_50V_TIMEOUT: Duration = Duration::from_millis(500);

struct CalibrationState {
    driver_dac_code: u16,
    final_dac_code: u16,
    calibrated_at: Option<Instant>,
    calibration_temp: i16,
    valid: bool,
}

impl CalibrationState {
    fn new() -> Self {
        Self {
            driver_dac_code: 0,
            final_dac_code: 0,
            calibrated_at: None,
            calibration_temp: 0,
            valid: false,
        }
    }
}

pub struct HfAmp {
    driver_dac: MCP4725<PeripherialI2c>,
    final_dac: MCP4725<PeripherialI2c>,
    adc: ADS1115<PeripherialI2c>,
    adc_initialized: bool,
    mode: Mode,
    cal: CalibrationState,
    budget_cp: i32,
    thermal_cp: i32,
    last_contract: PdContract,
    cordic: &'static CordicMutex,
}

impl HfAmp {
    pub fn new(
        i2c: PeripherialI2cMutex,
        driver_dac_addr: I2cAddress,
        final_dac_addr: I2cAddress,
        adc_addr: I2cAddress,
        cordic: &'static CordicMutex,
    ) -> Self {
        Self {
            driver_dac: MCP4725::new(driver_dac_addr.into(), i2c),
            final_dac: MCP4725::new(final_dac_addr.into(), i2c),
            adc: ADS1115::new(adc_addr.into(), ADS1115Config::default(), i2c),
            adc_initialized: false,
            mode: Mode::StandBy,
            cal: CalibrationState::new(),
            budget_cp: 10000,
            thermal_cp: 10000,
            last_contract: PdContract::default(),
            cordic,
        }
    }

    pub async fn set_mode(&mut self, mode: Mode) -> Result<(), &'static str> {
        self.mode = mode;
        match mode {
            Mode::StandBy => {
                self.driver_dac
                    .write_eeprom_power_down()
                    .await
                    .map_err(|_| "HfAmp: driver DAC power down failed")?;
                self.final_dac
                    .write_eeprom_power_down()
                    .await
                    .map_err(|_| "HfAmp: final DAC power down failed")?;
                Ok(())
            }
            Mode::WarmUp | Mode::Rx => {
                self.driver_dac
                    .set_raw(0)
                    .await
                    .map_err(|_| "HfAmp: driver DAC zero failed")?;
                self.final_dac
                    .set_raw(0)
                    .await
                    .map_err(|_| "HfAmp: final DAC zero failed")?;
                Ok(())
            }
            Mode::Tx => self.enter_tx().await,
        }
    }

    async fn enter_tx(&mut self) -> Result<(), &'static str> {
        let mut rail_rcv = RAIL_50V_READY.receiver().unwrap();
        match embassy_time::with_timeout(RAIL_50V_TIMEOUT, rail_rcv.changed()).await {
            Ok(true) => {}
            Ok(false) => return Err("HfAmp: 50V rail not ready"),
            Err(_) => return Err("HfAmp: 50V rail timeout"),
        }

        if self.needs_calibration() {
            self.calibrate().await?;
        }

        self.update_idq().await?;

        Timer::after_millis(50).await;

        let mut pa_reading_rcv = PA_CURRENT_READING.receiver().unwrap();
        PA_CURRENT_REQUEST.sender().send(());
        let total_idq = pa_reading_rcv.changed().await;
        if total_idq < 50 || total_idq > 500 {
            self.driver_dac.set_raw(0).await.map_err(|_| "HfAmp: DAC zero failed")?;
            self.final_dac.set_raw(0).await.map_err(|_| "HfAmp: DAC zero failed")?;
            self.cal.valid = false;
            return Err("HfAmp: IDQ verification failed");
        }

        Ok(())
    }

    fn needs_calibration(&self) -> bool {
        if !self.cal.valid {
            return true;
        }
        match self.cal.calibrated_at {
            None => true,
            Some(t) => t.elapsed().as_millis() > CALIBRATION_STALE_MS,
        }
    }

    async fn calibrate(&mut self) -> Result<(), &'static str> {
        self.driver_dac.set_raw(0).await.map_err(|_| "HfAmp: driver DAC zero failed")?;
        self.final_dac.set_raw(0).await.map_err(|_| "HfAmp: final DAC zero failed")?;
        Timer::after_millis(10).await;

        PA_FAST_MODE.sender().send(true);
        Timer::after_millis(5).await;

        let final_code =
            self.ramp_calibrate_stage(false, FINAL_TARGET_IDQ_MA, FINAL_TOLERANCE_MA).await?;

        self.final_dac.set_raw(0).await.map_err(|_| "HfAmp: final DAC zero failed")?;
        Timer::after_millis(5).await;

        let driver_code =
            self.ramp_calibrate_stage(true, DRIVER_TARGET_IDQ_MA, DRIVER_TOLERANCE_MA).await?;

        self.driver_dac
            .set_raw(driver_code)
            .await
            .map_err(|_| "HfAmp: driver DAC restore failed")?;
        self.final_dac
            .set_raw(final_code)
            .await
            .map_err(|_| "HfAmp: final DAC restore failed")?;

        PA_FAST_MODE.sender().send(false);

        self.cal.driver_dac_code = driver_code;
        self.cal.final_dac_code = final_code;
        self.cal.calibrated_at = Some(Instant::now());
        self.cal.valid = true;

        if self.adc_initialized {
            if let Ok(raw) = self.adc.read_ain0().await {
                self.cal.calibration_temp = self.raw_to_celsius(raw);
            }
        }

        Ok(())
    }

    async fn ramp_calibrate_stage(
        &mut self,
        is_driver: bool,
        target_ma: i32,
        tolerance_ma: i32,
    ) -> Result<u16, &'static str> {
        let mut dac_code: u16 = 0;
        let mut step = DAC_COARSE_STEP;
        let mut overshot = false;

        loop {
            dac_code = dac_code.saturating_add(step);
            if dac_code > 4095 {
                if is_driver {
                    self.driver_dac.set_raw(0).await.map_err(|_| "HfAmp: DAC failed")?;
                } else {
                    self.final_dac.set_raw(0).await.map_err(|_| "HfAmp: DAC failed")?;
                }
                return Err("HfAmp: calibration DAC maxed out");
            }

            if is_driver {
                self.driver_dac.set_raw(dac_code).await.map_err(|_| "HfAmp: DAC failed")?;
            } else {
                self.final_dac.set_raw(dac_code).await.map_err(|_| "HfAmp: DAC failed")?;
            }

            Timer::after_millis(5).await;

            let mut pa_reading_rcv = PA_CURRENT_READING.receiver().unwrap();
            PA_CURRENT_REQUEST.sender().send(());
            let current = pa_reading_rcv.changed().await;

            let diff = current - target_ma;

            if diff.abs() <= tolerance_ma {
                return Ok(dac_code);
            }

            if diff > 0 {
                if !overshot {
                    overshot = true;
                    dac_code = dac_code.saturating_sub(step);
                    step = DAC_FINE_STEP;
                } else {
                    dac_code = dac_code.saturating_sub(step);
                    return Ok(dac_code);
                }
            }
        }
    }

    pub async fn set_power_telemetry(
        &mut self,
        telemetry: PowerTelemetry,
    ) -> Result<(), &'static str> {
        self.budget_cp = telemetry.power_budget(&self.last_contract);
        self.update_idq().await
    }

    pub async fn set_pd_contract(&mut self, contract: PdContract) -> Result<(), &'static str> {
        self.last_contract = contract;
        self.update_idq().await
    }

    pub async fn set_thermal_constraint(&mut self, thermal: i32) -> Result<(), &'static str> {
        self.thermal_cp = thermal;
        self.update_idq().await
    }

    fn idq_scale(&self) -> u16 {
        let constraint = self.budget_cp.min(self.thermal_cp);
        if constraint >= 0 {
            10000
        } else {
            (10000 + constraint).max(0) as u16
        }
    }

    async fn update_idq(&mut self) -> Result<(), &'static str> {
        if self.mode != Mode::Tx {
            return Ok(());
        }

        let scale = self.idq_scale().min(10000) as u32;
        let driver_bias = (scale * self.cal.driver_dac_code as u32 / 10000) as u16;
        let final_bias = (scale * self.cal.final_dac_code as u32 / 10000) as u16;

        self.driver_dac
            .set_raw(driver_bias)
            .await
            .map_err(|_| "HfAmp: driver DAC write failed")?;
        self.final_dac
            .set_raw(final_bias)
            .await
            .map_err(|_| "HfAmp: final DAC write failed")?;

        Ok(())
    }

    pub async fn read_temperatures(&mut self) -> Result<PaTemperatures, &'static str> {
        if !self.adc_initialized {
            self.adc
                .init()
                .await
                .map_err(|_| "HfAmp: ADS1115 init failed")?;
            self.adc_initialized = true;
        }

        let driver_raw = self
            .adc
            .read_ain0()
            .await
            .map_err(|_| "HfAmp: read driver temp failed")?;
        let final_raw = self
            .adc
            .read_ain1()
            .await
            .map_err(|_| "HfAmp: read final temp failed")?;

        let driver_c = self.raw_to_celsius(driver_raw);
        let final_c = self.raw_to_celsius(final_raw);

        let worst_c = driver_c.max(final_c);
        if self.cal.valid && self.cal.calibration_temp != 0 {
            let drift = (worst_c - self.cal.calibration_temp).abs();
            if drift > CALIBRATION_TEMP_DRIFT_C {
                self.cal.valid = false;
            }
        }

        Ok(PaTemperatures {
            driver_c,
            final_c,
        })
    }

    pub fn is_thermal_emergency(temps: &PaTemperatures) -> bool {
        temps.driver_c >= THERMAL_EMERGENCY_C || temps.final_c >= THERMAL_EMERGENCY_C
    }

    pub fn compute_thermal_constraint(temps: &PaTemperatures) -> i32 {
        let worst = temps.driver_c.max(temps.final_c);
        if worst >= THERMAL_EMERGENCY_C {
            -10000
        } else if worst >= THERMAL_AD8367_ZERO_C {
            let range = (THERMAL_EMERGENCY_C - THERMAL_AD8367_ZERO_C) as i32;
            let above = (worst - THERMAL_AD8367_ZERO_C) as i32;
            -(above * 10000 / range)
        } else if worst > THERMAL_DERATING_START_C {
            let range = (THERMAL_AD8367_ZERO_C - THERMAL_DERATING_START_C) as i32;
            let above = (worst - THERMAL_DERATING_START_C) as i32;
            (range - above) * 10000 / range
        } else {
            10000
        }
    }

    pub async fn emergency_off(&mut self) {
        let _ = self.driver_dac.set_raw(0).await;
        let _ = self.final_dac.set_raw(0).await;
        self.cal.valid = false;
        self.mode = Mode::StandBy;
    }

    fn raw_to_celsius(&self, raw: i16) -> i16 {
        if raw <= 0 {
            return 150;
        }
        let voltage = raw as f32 * ADC_FSR / ADC_MAX;
        let r_ntc = PULL_UP_R * voltage / (VCC - voltage);
        if r_ntc <= 0.0 {
            return 150;
        }
        let ln_r_ratio = with_cordic(self.cordic, |c| c.lnf(r_ntc / NTC_R0));
        let t_kelvin = 1.0 / (1.0 / NTC_T0 + ln_r_ratio / NTC_B);
        (t_kelvin - 273.15) as i16
    }
}
