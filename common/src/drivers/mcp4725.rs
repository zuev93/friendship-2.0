/*
 * MCP4725 12-bit DAC Driver
 */

use core::result::Result;
use embassy_sync::mutex::Mutex;
use crate::PlatformMutex;
use embassy_time::Timer;
use embedded_hal_async::i2c::I2c;

pub struct MCP4725<I2C>
where
    I2C: I2c + 'static,
{
    address: u8,
    i2c: &'static Mutex<PlatformMutex, I2C>,
}

impl<I2C> MCP4725<I2C>
where
    I2C: I2c + 'static,
{
    pub fn new(address: u8, i2c: &'static Mutex<PlatformMutex, I2C>) -> Self {
        Self { address, i2c }
    }

    pub async fn set_raw(&self, value: u16) -> Result<(), I2C::Error> {
        let value = value & 0x0FFF;

        let byte1 = ((value >> 8) & 0x0F) as u8;
        let byte2 = (value & 0xFF) as u8;

        let data = [byte1, byte2];
        self.i2c.lock().await.write(self.address, &data).await?;

        Ok(())
    }

    pub async fn write_eeprom_power_down(&self) -> Result<(), I2C::Error> {
        let byte1 = 0b01110000;
        let byte2 = 0x00;

        let data = [byte1, byte2];
        self.i2c.lock().await.write(self.address, &data).await?;

        Timer::after_millis(50).await;

        Ok(())
    }
}
