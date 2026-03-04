use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use embedded_hal_async::i2c::I2c;

#[repr(u8)]
pub enum Register {
    ModeControl1 = 0x01,
    DacControl = 0x02,
    DacVolumeA = 0x03,
    DacVolumeB = 0x04,
    DacMixing = 0x05,
    AdcControl = 0x06,
    ModeControl2 = 0x07,
    ChipId = 0x08,
}

const MODE_CTRL2_POWER_DOWN_CP_EN: u8 = 0x03;
const MODE_CTRL2_CP_EN: u8 = 0x02;

const MODE_CTRL1_MASTER: u8 = 1 << 7;
const MODE_CTRL1_QUAD_SPEED: u8 = 0x03;
const MODE_CTRL1_RATIO_128: u8 = 0x00 << 5;

const DAC_I2S_24BIT: u8 = 0x09;
const ADC_I2S_24BIT: u8 = 0x00;

pub struct Cs4272<I2C>
where
    I2C: I2c + 'static,
{
    i2c_addr: u8,
    i2c: &'static Mutex<ThreadModeRawMutex, I2C>,
}

impl<I2C> Cs4272<I2C>
where
    I2C: I2c + 'static,
{
    pub fn new(i2c_addr: u8, i2c: &'static Mutex<ThreadModeRawMutex, I2C>) -> Self {
        Self { i2c_addr, i2c }
    }

    pub async fn write_register(&mut self, reg: Register, value: u8) -> Result<(), I2C::Error> {
        let map_byte = reg as u8;
        self.i2c
            .lock()
            .await
            .write(self.i2c_addr, &[map_byte, value])
            .await
    }

    pub async fn read_register(&mut self, reg: Register) -> Result<u8, I2C::Error> {
        let map_byte = reg as u8;
        let mut buffer = [0u8; 1];
        self.i2c
            .lock()
            .await
            .write_read(self.i2c_addr, &[map_byte], &mut buffer)
            .await?;
        Ok(buffer[0])
    }

    pub async fn init(&mut self) -> Result<(), I2C::Error> {
        self.write_register(Register::ModeControl2, MODE_CTRL2_POWER_DOWN_CP_EN)
            .await?;

        const MASTER_QUAD_128: u8 = MODE_CTRL1_MASTER | MODE_CTRL1_RATIO_128 | MODE_CTRL1_QUAD_SPEED;
        self.write_register(Register::ModeControl1, MASTER_QUAD_128)
            .await?;

        self.write_register(Register::DacControl, DAC_I2S_24BIT)
            .await?;

        self.write_register(Register::DacVolumeA, 0x00).await?;
        self.write_register(Register::DacVolumeB, 0x00).await?;

        self.write_register(Register::AdcControl, ADC_I2S_24BIT)
            .await?;

        self.write_register(Register::ModeControl2, MODE_CTRL2_CP_EN)
            .await?;

        Ok(())
    }
}
