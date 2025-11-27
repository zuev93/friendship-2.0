use core::result::Result;
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use embassy_time::Timer;
use embedded_hal_async::i2c::I2c;

const REG_CONVERSION: u8 = 0x00;
const REG_CONFIG: u8 = 0x01;

const CFG_OS_BUSY: u16 = 0x8000;

const CFG_MUX_AIN0_GND: u16 = 0x4000;
const CFG_MUX_AIN1_GND: u16 = 0x5000;
const CFG_MUX_AIN2_GND: u16 = 0x6000;
const CFG_MUX_AIN3_GND: u16 = 0x7000;

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub enum Gain {
    Gain6144mV = 0x0000,
    Gain4096mV = 0x0200,
    Gain2048mV = 0x0400,
    Gain1024mV = 0x0600,
    Gain512mV = 0x0800,
    Gain256mV = 0x0A00,
}

const CFG_MODE_CONTINUOUS: u16 = 0x0000;

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub enum DataRate {
    Sps8 = 0x0000,
    Sps16 = 0x0020,
    Sps32 = 0x0040,
    Sps64 = 0x0060,
    Sps128 = 0x0080,
    Sps250 = 0x00A0,
    Sps475 = 0x00C0,
    Sps860 = 0x00E0,
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub enum ComparatorQueue {
    After1 = 0x0000,
    After2 = 0x0001,
    After4 = 0x0002,
    Disable = 0x0003,
}

#[derive(Clone, Copy)]
pub struct ADS1115Config {
    pub gain: Gain,
    pub data_rate: DataRate,
    pub comp_queue: ComparatorQueue,
}

impl Default for ADS1115Config {
    fn default() -> Self {
        Self {
            gain: Gain::Gain4096mV,
            data_rate: DataRate::Sps128,
            comp_queue: ComparatorQueue::Disable,
        }
    }
}

pub struct ADS1115<I2C>
where
    I2C: I2c + 'static,
{
    address: u8,
    config: ADS1115Config,
    current_mux: u16,
    i2c: &'static Mutex<ThreadModeRawMutex, I2C>,
}

impl<I2C> ADS1115<I2C>
where
    I2C: I2c + 'static,
{
    pub fn new(
        address: u8,
        config: ADS1115Config,
        i2c: &'static Mutex<ThreadModeRawMutex, I2C>,
    ) -> Self {
        Self {
            address,
            config,
            current_mux: CFG_MUX_AIN0_GND,
            i2c,
        }
    }

    pub async fn init(&mut self) -> Result<(), I2C::Error> {
        let config_reg: u16 = CFG_MUX_AIN0_GND
            | (self.config.gain as u16)
            | CFG_MODE_CONTINUOUS
            | (self.config.data_rate as u16)
            | (self.config.comp_queue as u16);

        self.write_register(REG_CONFIG, config_reg).await?;
        Timer::after_millis(10).await;
        Ok(())
    }

    pub async fn read_ain0(&mut self) -> Result<i16, I2C::Error> {
        self.read_channel(CFG_MUX_AIN0_GND).await
    }

    pub async fn read_ain1(&mut self) -> Result<i16, I2C::Error> {
        self.read_channel(CFG_MUX_AIN1_GND).await
    }

    pub async fn read_ain2(&mut self) -> Result<i16, I2C::Error> {
        self.read_channel(CFG_MUX_AIN2_GND).await
    }

    pub async fn read_ain3(&mut self) -> Result<i16, I2C::Error> {
        self.read_channel(CFG_MUX_AIN3_GND).await
    }

    async fn read_channel(&mut self, mux: u16) -> Result<i16, I2C::Error> {
        if self.current_mux != mux {
            let config = mux
                | (self.config.gain as u16)
                | CFG_MODE_CONTINUOUS
                | (self.config.data_rate as u16)
                | (self.config.comp_queue as u16);

            self.write_register(REG_CONFIG, config).await?;
            self.current_mux = mux;

            self.wait_ready().await?;
        }

        self.read_register(REG_CONVERSION).await
    }

    async fn wait_ready(&self) -> Result<(), I2C::Error> {
        for _ in 0..100 {
            let config = self.read_register(REG_CONFIG).await?;
            if (config as u16) & CFG_OS_BUSY != 0 {
                return Ok(());
            }
            Timer::after_micros(100).await;
        }
        Ok(())
    }

    async fn write_register(&self, reg: u8, value: u16) -> Result<(), I2C::Error> {
        let data = [reg, (value >> 8) as u8, value as u8];
        self.i2c.lock().await.write(self.address, &data).await
    }

    async fn read_register(&self, reg: u8) -> Result<i16, I2C::Error> {
        let mut lock = self.i2c.lock().await;
        lock.write(self.address, &[reg]).await?;

        let mut buffer = [0u8; 2];
        lock.read(self.address, &mut buffer).await?;

        let value = ((buffer[0] as u16) << 8) | (buffer[1] as u16);
        Ok(value as i16)
    }
}
