use embassy_sync::mutex::Mutex;
use crate::PlatformMutex;
use embedded_hal_async::i2c::I2c;

const CMD_SINGLE_WRITE: u8 = 0x58;
const MCP4728_BASE_ADDR: u8 = 0x60;

#[derive(Clone, Copy, PartialEq)]
pub enum Channel {
    A = 0,
    B = 1,
    C = 2,
    D = 3,
}

pub struct MCP4728<I2C>
where
    I2C: I2c + 'static,
{
    address: u8,
    i2c: &'static Mutex<PlatformMutex, I2C>,
}

impl<I2C> MCP4728<I2C>
where
    I2C: I2c + 'static,
{
    pub fn new(address: u8, i2c: &'static Mutex<PlatformMutex, I2C>) -> Self {
        Self { address, i2c }
    }

    pub async fn fast_write(&self, values: [u16; 4]) -> Result<(), I2C::Error> {
        let mut buf = [0u8; 8];
        for (i, &val) in values.iter().enumerate() {
            let v = val & 0x0FFF;
            buf[i * 2] = (v >> 8) as u8;
            buf[i * 2 + 1] = (v & 0xFF) as u8;
        }
        self.i2c.lock().await.write(self.address, &buf).await
    }

    pub async fn single_write(&self, channel: Channel, value: u16) -> Result<(), I2C::Error> {
        let v = value & 0x0FFF;
        let cmd = CMD_SINGLE_WRITE | ((channel as u8) << 1);
        let buf = [cmd, (v >> 8) as u8, (v & 0xFF) as u8];
        self.i2c.lock().await.write(self.address, &buf).await
    }

    pub async fn probe(&self) -> bool {
        let mut buf = [0u8; 1];
        self.i2c
            .lock()
            .await
            .read(self.address, &mut buf)
            .await
            .is_ok()
    }

    pub async fn ensure_address(&mut self) -> Result<bool, I2C::Error> {
        if self.address == MCP4728_BASE_ADDR {
            return Ok(false);
        }

        if self.probe().await {
            return Ok(false);
        }

        let new_addr_bits = (self.address & 0x07) << 2;
        let old_addr_bits = (MCP4728_BASE_ADDR & 0x07) << 2;

        let cmd1 = 0b01100001 | old_addr_bits;
        let cmd2 = 0b01100010 | new_addr_bits;
        let cmd3 = 0b01100011 | new_addr_bits;

        self.i2c
            .lock()
            .await
            .write(MCP4728_BASE_ADDR, &[cmd1, cmd2, cmd3])
            .await?;

        Ok(true)
    }
}
