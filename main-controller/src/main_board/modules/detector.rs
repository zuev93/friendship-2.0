use crate::app::types::{Mode, RfPowerPercent};
use crate::control_board::events::{PdContract, PowerTelemetry};
use crate::i2c_map::I2cAddress;
use crate::main_board::types::{MainBoardI2C, MainBoardI2CMutex};
use common::drivers::mcp4725::MCP4725;
use common::drivers::pca9534::{Pin, PCA9534};

const IO_RX_PIN: Pin = Pin::Pin0;
const IO_TX_PIN: Pin = Pin::Pin1;

const DAC_12BIT_MAX: u32 = 4095;
const CENTIPERCENT_MAX: u32 = 10000;

pub struct Detector {
    io: PCA9534<MainBoardI2C>,
    gain_dac: MCP4725<MainBoardI2C>,
    mode: Mode,
    user_power: RfPowerPercent,
    budget_cp: i32,
    thermal_cp: i32,
    alc_cp: i32,
    last_contract: PdContract,
}

impl Detector {
    pub fn new(
        i2c: &'static MainBoardI2CMutex,
        pca9534_addr: I2cAddress,
        mcp4725_addr: I2cAddress,
    ) -> Self {
        let io = PCA9534::new(pca9534_addr.into(), i2c);
        let gain_dac = MCP4725::new(mcp4725_addr.into(), i2c);

        Self {
            io,
            gain_dac,
            mode: Mode::StandBy,
            user_power: RfPowerPercent::new(0),
            budget_cp: 10000,
            thermal_cp: 10000,
            alc_cp: 10000,
            last_contract: PdContract::default(),
        }
    }

    pub async fn set_mode(&mut self, mode: Mode) -> Result<(), &'static str> {
        self.mode = mode;
        self.update_state().await
    }

    pub async fn set_power(&mut self, power: RfPowerPercent) -> Result<(), &'static str> {
        self.user_power = power;
        self.update_gain().await
    }

    pub async fn set_power_telemetry(
        &mut self,
        telemetry: PowerTelemetry,
    ) -> Result<(), &'static str> {
        self.budget_cp = telemetry.power_budget(&self.last_contract);
        self.update_gain().await
    }

    pub async fn set_pd_contract(&mut self, contract: PdContract) -> Result<(), &'static str> {
        self.last_contract = contract;
        self.update_gain().await
    }

    pub async fn set_thermal_constraint(&mut self, thermal: i32) -> Result<(), &'static str> {
        self.thermal_cp = thermal;
        self.update_gain().await
    }

    pub async fn set_alc_constraint(&mut self, alc: i32) -> Result<(), &'static str> {
        self.alc_cp = alc;
        self.update_gain().await
    }

    fn power_constraint(&self) -> i32 {
        self.budget_cp.min(self.thermal_cp).min(self.alc_cp)
    }

    fn effective_power(&self) -> u16 {
        let limit = self.power_constraint().max(0) as u16;
        self.user_power.centipercent.min(limit)
    }

    async fn update_state(&mut self) -> Result<(), &'static str> {
        if self.mode == Mode::StandBy {
            return Ok(());
        }
        if self.mode == Mode::WarmUp {
            return self.init().await;
        }

        let mut port: u8 = 0;
        if self.mode == Mode::Rx {
            port |= IO_RX_PIN.mask();
        }
        if self.mode == Mode::Tx {
            port |= IO_TX_PIN.mask();
        }
        self.io
            .write_port(port)
            .await
            .map_err(|_| "Failed to write detector IO")?;

        self.update_gain().await
    }

    async fn update_gain(&mut self) -> Result<(), &'static str> {
        if self.mode == Mode::Tx {
            let dac_value =
                ((self.effective_power() as u32 * DAC_12BIT_MAX) / CENTIPERCENT_MAX) as u16;
            self.gain_dac
                .set_raw(dac_value)
                .await
                .map_err(|_| "Failed to set AD8367 gain")?;
        } else if self.mode == Mode::Rx {
            self.gain_dac
                .set_raw(2048)
                .await
                .map_err(|_| "Failed to set AD8367 default RX gain")?;
        } else {
            self.gain_dac
                .write_eeprom_power_down()
                .await
                .map_err(|_| "Failed to power down gain DAC")?;
        }
        Ok(())
    }

    pub async fn set_rx_gain_dac(&mut self, dac_value: u16) -> Result<(), &'static str> {
        if self.mode == Mode::Rx {
            self.gain_dac
                .set_raw(dac_value)
                .await
                .map_err(|_| "Failed to set AD8367 RX gain")?;
        }
        Ok(())
    }

    async fn init(&mut self) -> Result<(), &'static str> {
        self.io
            .init()
            .await
            .map_err(|_| "Detector IO init failed")?;
        self.io
            .set_direction(0x00)
            .await
            .map_err(|_| "Detector IO direction failed")?;
        Ok(())
    }
}
