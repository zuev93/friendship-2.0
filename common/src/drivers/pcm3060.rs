use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use embedded_hal_async::i2c::I2c;

#[repr(u8)]
pub enum Register {
    Reset = 0x00,
    AdcControl = 0x01,
    DacControl = 0x02,
    AdcInput = 0x03,
    DacOutput = 0x04,
    AdcAnalog = 0x05,
    DacAnalog = 0x06,
    AdcDigital1 = 0x07,
    AdcDigital2 = 0x08,
    DacDigital1 = 0x09,
    DacDigital2 = 0x0A,
    AdcDigital3 = 0x0B,
    DacDigital3 = 0x0C,
    AdcDigital4 = 0x0D,
    DacDigital4 = 0x0E,
    AdcDigital5 = 0x0F,
    DacDigital5 = 0x10,
}

pub struct Pcm3060<I2C>
where
    I2C: I2c + 'static,
{
    i2c_addr: u8,
    i2c: &'static Mutex<ThreadModeRawMutex, I2C>,
}

impl<I2C> Pcm3060<I2C>
where
    I2C: I2c + 'static,
{
    pub fn new(i2c_addr: u8, i2c: &'static Mutex<ThreadModeRawMutex, I2C>) -> Self {
        Self { i2c_addr, i2c }
    }

    pub async fn write_register(&mut self, reg: Register, value: u8) -> Result<(), I2C::Error> {
        let reg_addr = reg as u8;
        self.i2c
            .lock()
            .await
            .write(self.i2c_addr, &[reg_addr, value])
            .await
    }

    pub async fn read_register(&mut self, reg: Register) -> Result<u8, I2C::Error> {
        let reg_addr = reg as u8;
        let mut buffer = [0u8; 1];
        self.i2c
            .lock()
            .await
            .write_read(self.i2c_addr, &[reg_addr], &mut buffer)
            .await?;
        Ok(buffer[0])
    }

    pub async fn reset(&mut self) -> Result<(), I2C::Error> {
        self.write_register(Register::Reset, 0x00).await
    }

    pub async fn init(&mut self) -> Result<(), I2C::Error> {
        self.reset().await?;
        Ok(())
    }
}
