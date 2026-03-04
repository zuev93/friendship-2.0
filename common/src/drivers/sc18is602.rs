/*
 * SC18IS602 I2C-to-SPI Bridge Driver (async with DMA)
 *
 * This chip acts as a bridge between I2C bus and SPI devices
 * Features:
 * - 200 byte buffer for SPI transactions
 * - 4 GPIO pins
 * - SPI clock up to 1.8 MHz (independent of I2C clock)
 */

use core::fmt;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Timer;
use embedded_hal_async::i2c::I2c;
use embedded_hal_async::spi::{ErrorKind, ErrorType, SpiBus};

const CMD_GPIO_DIR: u8 = 0x10;
const CMD_GPIO_WRITE: u8 = 0x11;
const CMD_GPIO_READ: u8 = 0x12;

#[derive(Debug)]
pub enum SC18IS602Error<E> {
    I2c(E),
}

impl<E: fmt::Debug> embedded_hal::spi::Error for SC18IS602Error<E> {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Other
    }
}

impl<E> From<E> for SC18IS602Error<E> {
    fn from(err: E) -> Self {
        SC18IS602Error::I2c(err)
    }
}

const REG_FUNCTION_ID: u8 = 0xF0;

#[derive(Debug, Clone, Copy)]
pub enum SPIClockSpeed {
    Rate1843kHz = 0b00,
}

#[derive(Debug, Clone, Copy)]
pub enum SPIMode {
    Mode0 = 0b00,
}

#[derive(Debug, Clone, Copy)]
pub enum SPIOrder {
    MsbFirst = 0,
}

pub struct SC18IS602Config {
    pub spi_mode: SPIMode,
    pub clock_speed: SPIClockSpeed,
    pub bit_order: SPIOrder,
}

impl Default for SC18IS602Config {
    fn default() -> Self {
        Self {
            spi_mode: SPIMode::Mode0,
            clock_speed: SPIClockSpeed::Rate1843kHz, // Maximum speed
            bit_order: SPIOrder::MsbFirst,
        }
    }
}

pub struct SC18IS602<I2C>
where
    I2C: I2c + 'static,
{
    address: u8,
    i2c: &'static Mutex<ThreadModeRawMutex, I2C>,
}

#[derive(Debug, Clone, Copy)]
pub enum GpioPin {
    P0 = 0,
    P1 = 1,
    P2 = 2,
    P3 = 3,
}

impl GpioPin {
    const fn mask(self) -> u8 {
        1 << (self as u8)
    }
}

impl<I2C> SC18IS602<I2C>
where
    I2C: I2c + 'static,
{
    pub fn new(address: u8, i2c: &'static Mutex<ThreadModeRawMutex, I2C>) -> Self {
        Self { address, i2c }
    }

    pub async fn init(&self, config: &SC18IS602Config) -> Result<(), I2C::Error> {
        let function_id: u8 = ((config.bit_order as u8) << 5)
            | ((config.spi_mode as u8) << 2)
            | (config.clock_speed as u8);

        let data = [REG_FUNCTION_ID, function_id];
        self.i2c.lock().await.write(self.address, &data).await?;

        Ok(())
    }

    pub async fn spi_transfer_ss0(&self, data: &[u8]) -> Result<(), I2C::Error> {
        self.spi_transfer_ss(0x01, data).await
    }

    pub async fn spi_transfer_ss1(&self, data: &[u8]) -> Result<(), I2C::Error> {
        self.spi_transfer_ss(0x02, data).await
    }

    pub async fn spi_transfer_ss2(&self, data: &[u8]) -> Result<(), I2C::Error> {
        self.spi_transfer_ss(0x04, data).await
    }

    pub async fn spi_transfer_ss3(&self, data: &[u8]) -> Result<(), I2C::Error> {
        self.spi_transfer_ss(0x08, data).await
    }

    async fn spi_transfer_ss(&self, ss_cmd: u8, data: &[u8]) -> Result<(), I2C::Error> {
        if data.is_empty() || data.len() > 200 {
            return Ok(());
        }

        let mut buffer = [0u8; 201];
        buffer[0] = ss_cmd;
        buffer[1..1 + data.len()].copy_from_slice(data);

        self.i2c
            .lock()
            .await
            .write(self.address, &buffer[..1 + data.len()])
            .await?;

        Ok(())
    }

    pub async fn spi_read(&self, buffer: &mut [u8]) -> Result<(), I2C::Error> {
        self.i2c.lock().await.read(self.address, buffer).await?;
        Ok(())
    }

    /// Configures GPIO direction bits. `1` sets the pin as output, `0` as input.
    pub async fn set_gpio_direction(&self, outputs_mask: u8) -> Result<(), I2C::Error> {
        let payload = [CMD_GPIO_DIR, outputs_mask & 0x0F];
        self.i2c.lock().await.write(self.address, &payload).await?;
        Ok(())
    }

    /// Writes the GPIO output levels. Only pins configured as outputs are affected.
    pub async fn write_gpio(&self, states: u8) -> Result<(), I2C::Error> {
        let payload = [CMD_GPIO_WRITE, states & 0x0F];
        self.i2c.lock().await.write(self.address, &payload).await?;
        Ok(())
    }

    /// Reads current GPIO levels (returns lower 4 bits).
    pub async fn read_gpio(&self) -> Result<u8, I2C::Error> {
        let mut guard = self.i2c.lock().await;
        guard.write(self.address, &[CMD_GPIO_READ]).await?;

        let mut buffer = [0u8; 1];
        guard.read(self.address, &mut buffer).await?;
        Ok(buffer[0] & 0x0F)
    }

    /// Convenience helper to set a single pin high/low without disturbing others.
    /// The pin must already be configured as an output.
    pub async fn write_gpio_pin(&self, pin: GpioPin, high: bool) -> Result<(), I2C::Error> {
        let mut states = self.read_gpio().await?;
        if high {
            states |= pin.mask();
        } else {
            states &= !pin.mask();
        }
        self.write_gpio(states).await
    }
}

pub struct SC18IS602SpiDevice<I2C: I2c + 'static> {
    pub bridge: SC18IS602<I2C>,
    ss_pin: u8, // Which SS pin to use (0-3)
}

impl<I2C: I2c + 'static> SC18IS602SpiDevice<I2C> {
    pub fn new(bridge: SC18IS602<I2C>, ss_pin: u8) -> Self {
        Self { bridge, ss_pin }
    }

    pub async fn init(&mut self) -> Result<(), I2C::Error> {
        self.bridge.init(&SC18IS602Config::default()).await?;
        Ok(())
    }
}

impl<I2C: I2c> ErrorType for SC18IS602SpiDevice<I2C> {
    type Error = SC18IS602Error<I2C::Error>;
}

impl<I2C: I2c> SpiBus for SC18IS602SpiDevice<I2C> {
    async fn read(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        self.bridge
            .spi_read(words)
            .await
            .map_err(SC18IS602Error::I2c)
    }

    async fn write(&mut self, words: &[u8]) -> Result<(), Self::Error> {
        let result = match self.ss_pin {
            0 => self.bridge.spi_transfer_ss0(words).await,
            1 => self.bridge.spi_transfer_ss1(words).await,
            2 => self.bridge.spi_transfer_ss2(words).await,
            3 => self.bridge.spi_transfer_ss3(words).await,
            _ => Ok(()), // Invalid SS pin - no-op
        };
        result.map_err(SC18IS602Error::I2c)
    }

    async fn transfer(&mut self, read: &mut [u8], write: &[u8]) -> Result<(), Self::Error> {
        self.write(write).await?;
        Timer::after_millis(1).await;
        if !read.is_empty() {
            self.read(read).await?;
        }
        Ok(())
    }

    async fn transfer_in_place(&mut self, words: &mut [u8]) -> Result<(), Self::Error> {
        const MAX_BUF: usize = 256;
        let len = words.len();

        if len > MAX_BUF {
            return Ok(());
        }

        let mut temp = [0u8; MAX_BUF];
        temp[..len].copy_from_slice(words);

        self.write(&temp[..len]).await?;
        Timer::after_millis(1).await;
        self.read(words).await
    }

    async fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
