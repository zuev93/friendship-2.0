use embassy_time::{Duration, Instant, Timer};

use crate::app::types::{Mode, PaTemperatures, RfPowerPercent};
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
const DRIVER_TARGET_IDQ_MA: i32 = 125;
const DRIVER_TOLERANCE_MA: i32 = 25;
const CALIBRATION_STALE_MS: u64 = 60_000;
const CALIBRATION_TEMP_DRIFT: i16 = 12_000;
const DAC_COARSE_STEP: u16 = 8;
const DAC_FINE_STEP: u16 = 2;

const THERMAL_DERATING_START: i16 = 20000;
const THERMAL_SHUTDOWN: i16 = 30000;

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
    user_power: RfPowerPercent,
    power_budget_limit: RfPowerPercent,
    thermal_limit: RfPowerPercent,
    cal: CalibrationState,
}

impl HfAmp {
    pub fn new(
        i2c: PeripherialI2cMutex,
        driver_dac_addr: I2cAddress,
        final_dac_addr: I2cAddress,
        adc_addr: I2cAddress,
    ) -> Self {
        Self {
            driver_dac: MCP4725::new(driver_dac_addr.into(), i2c),
            final_dac: MCP4725::new(final_dac_addr.into(), i2c),
            adc: ADS1115::new(adc_addr.into(), ADS1115Config::default(), i2c),
            adc_initialized: false,
            mode: Mode::StandBy,
            user_power: RfPowerPercent::new(10000),
            power_budget_limit: RfPowerPercent::new(10000),
            thermal_limit: RfPowerPercent::new(10000),
            cal: CalibrationState::new(),
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
        match embassy_time::with_timeout(RAIL_50V_TIMEOUT, RAIL_50V_READY.wait()).await {
            Ok(true) => {}
            Ok(false) => return Err("HfAmp: 50V rail not ready"),
            Err(_) => return Err("HfAmp: 50V rail timeout"),
        }

        if self.needs_calibration() {
            self.calibrate().await?;
        }

        self.update_bias().await?;

        Timer::after_millis(50).await;

        PA_CURRENT_REQUEST.signal(());
        let total_idq = PA_CURRENT_READING.wait().await;
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

        PA_FAST_MODE.signal(true);
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

        PA_FAST_MODE.signal(false);

        self.cal.driver_dac_code = driver_code;
        self.cal.final_dac_code = final_code;
        self.cal.calibrated_at = Some(Instant::now());
        self.cal.valid = true;

        if self.adc_initialized {
            if let Ok(temp) = self.adc.read_ain0().await {
                self.cal.calibration_temp = temp;
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

            PA_CURRENT_REQUEST.signal(());
            let current = PA_CURRENT_READING.wait().await;

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

    pub async fn set_user_power(&mut self, power: RfPowerPercent) -> Result<(), &'static str> {
        self.user_power = power;
        self.update_bias().await
    }

    pub async fn set_power_budget(&mut self, limit: RfPowerPercent) -> Result<(), &'static str> {
        self.power_budget_limit = limit;
        self.update_bias().await
    }

    pub fn derive_power_budget(telemetry: &PowerTelemetry, contract: &PdContract) -> RfPowerPercent {
        let max_current = contract.current_ma;
        let current = telemetry.vbus_current_ma.max(0) as u32;
        let headroom_ma = max_current.saturating_sub(current);
        let centipercent = if max_current > 0 {
            ((headroom_ma * 10000) / max_current) as u16
        } else {
            0
        };
        RfPowerPercent::new(centipercent)
    }

    pub async fn read_and_update_temperatures(
        &mut self,
    ) -> Result<PaTemperatures, &'static str> {
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

        let worst = driver_raw.max(final_raw);
        let old_limit = self.thermal_limit;

        self.thermal_limit = if worst >= THERMAL_SHUTDOWN {
            RfPowerPercent::new(0)
        } else if worst > THERMAL_DERATING_START {
            let range = (THERMAL_SHUTDOWN - THERMAL_DERATING_START) as u32;
            let above = (worst - THERMAL_DERATING_START) as u32;
            let cp = ((range - above) * 10000) / range;
            RfPowerPercent::new(cp as u16)
        } else {
            RfPowerPercent::new(10000)
        };

        if self.cal.valid && self.cal.calibration_temp != 0 {
            let drift = (worst - self.cal.calibration_temp).abs();
            if drift > CALIBRATION_TEMP_DRIFT {
                self.cal.valid = false;
            }
        }

        if self.thermal_limit != old_limit {
            let _ = self.update_bias().await;
        }

        Ok(PaTemperatures {
            driver_raw,
            final_raw,
        })
    }

    pub fn is_thermal_emergency(temps: &PaTemperatures) -> bool {
        temps.driver_raw >= THERMAL_SHUTDOWN || temps.final_raw >= THERMAL_SHUTDOWN
    }

    pub async fn emergency_off(&mut self) {
        let _ = self.driver_dac.set_raw(0).await;
        let _ = self.final_dac.set_raw(0).await;
        self.cal.valid = false;
        self.mode = Mode::StandBy;
    }

    async fn update_bias(&mut self) -> Result<(), &'static str> {
        if self.mode != Mode::Tx {
            return Ok(());
        }

        let effective = self
            .user_power
            .centipercent
            .min(self.power_budget_limit.centipercent)
            .min(self.thermal_limit.centipercent);

        let driver_bias = ((effective as u32 * self.cal.driver_dac_code as u32) / 10000) as u16;
        let final_bias = ((effective as u32 * self.cal.final_dac_code as u32) / 10000) as u16;

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
}
