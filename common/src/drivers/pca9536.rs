use embassy_sync::mutex::Mutex;
use crate::PlatformMutex;
use embedded_hal_async::i2c::I2c;

const REG_INPUT: u8 = 0x00;
const REG_OUTPUT: u8 = 0x01;
const REG_POLARITY: u8 = 0x02;
const REG_CONFIG: u8 = 0x03;

const PORT_MASK: u8 = 0x0F;

#[derive(Clone, Copy, PartialEq)]
pub enum Pin {
    IO0 = 0,
    IO1 = 1,
    IO2 = 2,
    IO3 = 3,
}

impl Pin {
    fn mask(self) -> u8 {
        1 << (self as u8)
    }
}

pub struct PCA9536<I2C>
where
    I2C: I2c + 'static,
{
    address: u8,
    i2c: &'static Mutex<PlatformMutex, I2C>,
}

impl<I2C> PCA9536<I2C>
where
    I2C: I2c + 'static,
{
    pub fn new(address: u8, i2c: &'static Mutex<PlatformMutex, I2C>) -> Self {
        Self { address, i2c }
    }

    pub async fn init(&mut self, config: u8) -> Result<(), I2C::Error> {
        self.write_reg(REG_CONFIG, config & PORT_MASK).await?;
        self.write_reg(REG_POLARITY, 0x00).await?;
        self.write_reg(REG_OUTPUT, 0x00).await?;
        Ok(())
    }

    pub async fn read_pin(&self, pin: Pin) -> Result<bool, I2C::Error> {
        let val = self.read_reg(REG_INPUT).await?;
        Ok((val & pin.mask()) != 0)
    }

    pub async fn write_pin(&self, pin: Pin, state: bool) -> Result<(), I2C::Error> {
        let mut val = self.read_reg(REG_OUTPUT).await?;
        if state {
            val |= pin.mask();
        } else {
            val &= !pin.mask();
        }
        self.write_reg(REG_OUTPUT, val & PORT_MASK).await
    }

    async fn write_reg(&self, reg: u8, val: u8) -> Result<(), I2C::Error> {
        self.i2c.lock().await.write(self.address, &[reg, val]).await
    }

    async fn read_reg(&self, reg: u8) -> Result<u8, I2C::Error> {
        let mut lock = self.i2c.lock().await;
        lock.write(self.address, &[reg]).await?;
        let mut buf = [0u8];
        lock.read(self.address, &mut buf).await?;
        Ok(buf[0] & PORT_MASK)
    }
}
