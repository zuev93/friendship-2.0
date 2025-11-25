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

pub struct Pcm3060 {
    i2c_addr: u8,
}

impl Pcm3060 {
    pub fn new(i2c_addr: u8) -> Self {
        Self { i2c_addr }
    }

    pub async fn write_register<I2C: I2c>(
        &mut self,
        i2c: &mut I2C,
        reg: Register,
        value: u8,
    ) -> Result<(), I2C::Error> {
        let reg_addr = reg as u8;
        i2c.write(self.i2c_addr, &[reg_addr, value]).await
    }

    pub async fn read_register<I2C: I2c>(
        &mut self,
        i2c: &mut I2C,
        reg: Register,
    ) -> Result<u8, I2C::Error> {
        let reg_addr = reg as u8;
        let mut buffer = [0u8; 1];
        i2c.write_read(self.i2c_addr, &[reg_addr], &mut buffer)
            .await?;
        Ok(buffer[0])
    }

    pub async fn reset<I2C: I2c>(&mut self, i2c: &mut I2C) -> Result<(), I2C::Error> {
        self.write_register(i2c, Register::Reset, 0x00).await
    }

    pub async fn init<I2C: I2c>(&mut self, i2c: &mut I2C) -> Result<(), I2C::Error> {
        self.reset(i2c).await?;
        Ok(())
    }
}
