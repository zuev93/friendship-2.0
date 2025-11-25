/*
 * TX Power Control Module
 *
 * Controls transmit power through MCP4725 DAC → power amplifier.
 *
 * Note: TX power control is only active during transmit (TX) mode.
 * During RX, this module is disabled to prevent accidental TX.
 */

use crate::app::types::{Mode, RfPowerPercent};
use common::drivers::mcp4725::MCP4725;
use crate::i2c_map;
use crate::main_board::types::MainBoardI2CMutex;

const DAC_ADDRESS: u8 = i2c_map::MCP4725_TX_POWER_ADDR;

pub struct TxPowerControl {
    dac: MCP4725,
    i2c: &'static MainBoardI2CMutex,
    desired_power: RfPowerPercent,
    mode: Mode,
}

impl TxPowerControl {
    pub fn new(i2c: &'static MainBoardI2CMutex, power: RfPowerPercent) -> Self {
        let dac = MCP4725::new(DAC_ADDRESS);
        Self {
            i2c,
            dac,
            desired_power: power,
            mode: Mode::StandBy,
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

    async fn update_state(&mut self) -> Result<(), &'static str> {
        let mut i2c_guard = self.i2c.lock().await;
        if self.mode == Mode::Tx {
            // TODO check max voltage
            let dac_value = ((self.desired_power.centipercent as u32 * 4095) / 10000) as u16;
            self.dac
                .set_raw(&mut *i2c_guard, dac_value)
                .await
                .map_err(|_| "Failed to set TX power")
        } else {
            self.dac
                .write_eeprom_power_down(&mut *i2c_guard)
                .await
                .map_err(|_| "Failed to write EEPROM power down")
        }
    }
}
