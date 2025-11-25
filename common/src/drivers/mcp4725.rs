/*
 * MCP4725 12-bit DAC Driver
 */

use core::result::Result;
use embassy_time::Timer;
use embedded_hal_async::i2c::I2c;

pub struct MCP4725 {
    address: u8,
}

impl MCP4725 {
    pub fn new(address: u8) -> Self {
        Self { address }
    }

    pub async fn set_raw<I2C: I2c>(&self, i2c: &mut I2C, value: u16) -> Result<(), I2C::Error> {
        let value = value & 0x0FFF;

        let byte1 = ((value >> 8) & 0x0F) as u8;
        let byte2 = (value & 0xFF) as u8;

        let data = [byte1, byte2];
        i2c.write(self.address, &data).await?;

        Ok(())
    }

    pub async fn write_eeprom_power_down<I2C: I2c>(&self, i2c: &mut I2C) -> Result<(), I2C::Error> {
        let byte1 = 0b01110000;
        let byte2 = 0x00;

        let data = [byte1, byte2];
        i2c.write(self.address, &data).await?;

        Timer::after_millis(50).await;

        Ok(())
    }
}
