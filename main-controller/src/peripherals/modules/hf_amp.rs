use crate::app::types::{Mode, PaTemperatures, RfPowerPercent};
use crate::control_board::events::PowerTelemetry;
use crate::i2c_map::I2cAddress;
use crate::peripherals::types::{PeripherialI2c, PeripherialI2cMutex};
use common::drivers::ads1115::{ADS1115, ADS1115Config};
use common::drivers::mcp4725::MCP4725;

const DRIVER_IDQ_MA: u16 = 100;
const FINAL_IDQ_MA: u16 = 100;
const DAC_COUNTS_PER_MA: u16 = 29;

const THERMAL_DERATING_START: i16 = 20000;
const THERMAL_SHUTDOWN: i16 = 30000;

const VCC_MAX_CURRENT_MA: u16 = 5000;

pub struct HfAmp {
    driver_dac: MCP4725<PeripherialI2c>,
    final_dac: MCP4725<PeripherialI2c>,
    adc: ADS1115<PeripherialI2c>,
    adc_initialized: bool,
    mode: Mode,
    user_power: RfPowerPercent,
    power_budget_limit: RfPowerPercent,
    thermal_limit: RfPowerPercent,
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
            Mode::Tx => self.update_bias().await,
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

    pub fn derive_power_budget(telemetry: &PowerTelemetry) -> RfPowerPercent {
        let current = telemetry.vbus_current_ma.max(0) as u32;
        let headroom_ma = (VCC_MAX_CURRENT_MA as u32).saturating_sub(current);
        let centipercent = ((headroom_ma * 10000) / VCC_MAX_CURRENT_MA as u32) as u16;
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

        if self.thermal_limit != old_limit {
            let _ = self.update_bias().await;
        }

        Ok(PaTemperatures {
            driver_raw,
            final_raw,
        })
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

        let driver_max = DRIVER_IDQ_MA as u32 * DAC_COUNTS_PER_MA as u32;
        let final_max = FINAL_IDQ_MA as u32 * DAC_COUNTS_PER_MA as u32;
        let driver_bias = ((effective as u32 * driver_max) / 10000) as u16;
        let final_bias = ((effective as u32 * final_max) / 10000) as u16;

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
