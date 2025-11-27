/*
 * TX Power Control Module
 *
 * Controls transmit power through MCP4725 DAC → power amplifier.
 *
 * Note: TX power control is only active during transmit (TX) mode.
 * During RX, this module is disabled to prevent accidental TX.
 */

use crate::app::types::{FilterType, Mode, RfGainMode, RfPowerPercent};
use crate::i2c_map::{self, FILTER_PCA9534_ADDR};
use crate::main_board::types::{MainBoardI2C, MainBoardI2CMutex};
use common::drivers::mcp4725::MCP4725;
use common::drivers::pca9534::{Pin, PCA9534};

const DAC_ADDRESS: u8 = i2c_map::MCP4725_TX_POWER_ADDR;
const IO_RX_PIN: Pin = Pin::Pin0;
const IO_TX_PIN: Pin = Pin::Pin1;
const IO_F1_PIN: Pin = Pin::Pin2;
const IO_F2_PIN: Pin = Pin::Pin3;
const IO_AMP_EN_PIN: Pin = Pin::Pin4;
const IO_AMP_OFF_PIN: Pin = Pin::Pin5;

pub struct CrystallFilter {
    dac: MCP4725<MainBoardI2C>,
    io: PCA9534<MainBoardI2C>,
    desired_power: RfPowerPercent,
    mode: Mode,
    filter_type: FilterType,
    rf_gain_mode: RfGainMode,
}

impl CrystallFilter {
    pub fn new(i2c: &'static MainBoardI2CMutex) -> Self {
        let dac = MCP4725::new(DAC_ADDRESS, i2c);
        let io = PCA9534::new(FILTER_PCA9534_ADDR, i2c);
        Self {
            io,
            dac,
            desired_power: RfPowerPercent::new(0),
            mode: Mode::StandBy,
            filter_type: FilterType::Single,
            rf_gain_mode: RfGainMode::Attenuator,
        }
    }

    pub async fn set_power(&mut self, power: RfPowerPercent) -> Result<(), &'static str> {
        self.desired_power = power;
        self.update_state().await
    }

    pub async fn set_mode(&mut self, mode: Mode) -> Result<(), &'static str> {
        self.mode = mode;
        self.update_state().await
    }

    pub async fn set_filter_type(&mut self, filter_type: FilterType) -> Result<(), &'static str> {
        self.filter_type = filter_type;
        self.update_state().await
    }

    pub async fn set_rf_gain_mode(&mut self, rf_gain_mode: RfGainMode) -> Result<(), &'static str> {
        self.rf_gain_mode = rf_gain_mode;
        self.update_state().await
    }

    async fn update_state(&mut self) -> Result<(), &'static str> {
        if self.mode == Mode::WarmUp {
            self.init().await?;
        } else if self.mode == Mode::StandBy {
            return Ok(());
        }
        let mut port: u8 = 0;

        if self.mode == Mode::Rx {
            port |= IO_RX_PIN.mask();
        }
        if self.mode == Mode::Tx {
            port |= IO_TX_PIN.mask();
        }
        if self.filter_type == FilterType::Single {
            port |= IO_F2_PIN.mask();
        } else {
            port |= IO_F1_PIN.mask();
        }
        let amp_enabled =
            self.rf_gain_mode == RfGainMode::RfSingle || self.rf_gain_mode == RfGainMode::RfDouble;
        if amp_enabled {
            port |= IO_AMP_EN_PIN.mask();
        } else {
            port |= IO_AMP_OFF_PIN.mask();
        }

        self.io
            .write_port(port)
            .await
            .map_err(|_| "Failed to write filter IO")?;

        if self.mode == Mode::Tx {
            // TODO check max voltage
            let dac_value = ((self.desired_power.centipercent as u32 * 4095) / 10000) as u16;
            self.dac
                .set_raw(dac_value)
                .await
                .map_err(|_| "Failed to set TX power")?;
        } else {
            self.dac
                .write_eeprom_power_down()
                .await
                .map_err(|_| "Failed to write EEPROM power down")?;
        }
        Ok(())
    }

    async fn init(&mut self) -> Result<(), &'static str> {
        self.io
            .init()
            .await
            .map_err(|_| "Failed to init crystal filter IO")?;
        self.io
            .set_direction(0x0)
            .await
            .map_err(|_| "Failed to init crystal filter IO")?;
        Ok(())
    }
}
