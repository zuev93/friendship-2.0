use core::result::Result;
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use embassy_time::Timer;
use embedded_hal_async::i2c::I2c;

const REG_CONVERSION: u8 = 0x00;
const REG_CONFIG: u8 = 0x01;

const CFG_OS_BUSY: u16 = 0x8000;

const CFG_MUX_AIN0_GND: u16 = 0x4000;
const CFG_MUX_AIN1_GND: u16 = 0x5000;

const CFG_PGA_4096MV: u16 = 0x0200;

const CFG_MODE_CONTINUOUS: u16 = 0x0000;

const CFG_DR_3300SPS: u16 = 0x00C0;

const CFG_COMP_QUE_DISABLE: u16 = 0x0003;

pub struct ADS1015<I2C>
where
    I2C: I2c + 'static,
{
    address: u8,
    current_mux: u16,
    i2c: &'static Mutex<ThreadModeRawMutex, I2C>,
}

impl<I2C> ADS1015<I2C>
where
    I2C: I2c + 'static,
{
    pub fn new(address: u8, i2c: &'static Mutex<ThreadModeRawMutex, I2C>) -> Self {
        Self {
            address,
            current_mux: CFG_MUX_AIN0_GND,
            i2c,
        }
    }

    pub async fn init(&mut self) -> Result<(), I2C::Error> {
        let config_reg: u16 = CFG_MUX_AIN0_GND
            | CFG_PGA_4096MV
            | CFG_MODE_CONTINUOUS
            | CFG_DR_3300SPS
            | CFG_COMP_QUE_DISABLE;

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

    async fn read_channel(&mut self, mux: u16) -> Result<i16, I2C::Error> {
        if self.current_mux != mux {
            let config = mux
                | CFG_PGA_4096MV
                | CFG_MODE_CONTINUOUS
                | CFG_DR_3300SPS
                | CFG_COMP_QUE_DISABLE;

            self.write_register(REG_CONFIG, config).await?;
            self.current_mux = mux;

            self.wait_ready().await?;
        }

        let raw = self.read_register(REG_CONVERSION).await?;
        Ok(raw >> 4)
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
