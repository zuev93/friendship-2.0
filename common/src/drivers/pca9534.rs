/*
 * PCA9534PWR 8-bit I2C I/O Expander Driver
 *
 * Features:
 * - Single 8-bit port
 * - Configurable as inputs or outputs per pin
 * - Polarity inversion support
 * - I2C interface up to 400 kHz
 * - Interrupt output (INT) - active low, open-drain
 * - Low standby current
 *
 * Interrupt Behavior:
 * - INT pin goes LOW when input state differs from port value
 * - Reading input register clears the interrupt
 * - Useful for event-driven input handling instead of polling
 *
 * Pin Configuration:
 * - Configuration register: 1 = input (high-Z), 0 = output
 * - Default: all pins configured as inputs
 *
 * Usage Example:
 * ```rust
 * let mut gpio = PCA9534::new(0x20);
 * gpio.init(&mut i2c).await?;
 *
 * // Configure pins 0-3 as outputs, 4-7 as inputs
 * gpio.set_direction(&mut i2c, 0xF0).await?;
 *
 * // Set pin 0 high
 * gpio.write_pin(&mut i2c, Pin::Pin0, true).await?;
 *
 * // Read pin 4
 * let state = gpio.read_pin(&mut i2c, Pin::Pin4).await?;
 *
 * // Read all pins and clear interrupt
 * let port_state = gpio.read_and_clear_int(&mut i2c).await?;
 * ```
 */

use core::result::Result;
use embassy_sync::mutex::Mutex;
use crate::PlatformMutex;
use embedded_hal_async::i2c::I2c;

const REG_INPUT_PORT: u8 = 0x00;
const REG_OUTPUT_PORT: u8 = 0x01;
const REG_POLARITY_INV: u8 = 0x02;
const REG_CONFIG: u8 = 0x03;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pin {
    Pin0 = 0,
    Pin1 = 1,
    Pin2 = 2,
    Pin3 = 3,
    Pin4 = 4,
    Pin5 = 5,
    Pin6 = 6,
    Pin7 = 7,
}

impl Pin {
    pub fn mask(self) -> u8 {
        1 << (self as u8)
    }
}

pub struct PCA9534<I2C>
where
    I2C: I2c + 'static,
{
    address: u8,
    i2c: &'static Mutex<PlatformMutex, I2C>,
}

impl<I2C> PCA9534<I2C>
where
    I2C: I2c + 'static,
{
    pub fn new(address: u8, i2c: &'static Mutex<PlatformMutex, I2C>) -> Self {
        Self { address, i2c }
    }

    /// Initialize the PCA9534 with default configuration
    /// - All pins as inputs
    /// - No polarity inversion
    /// - Output register cleared
    pub async fn init(&mut self) -> Result<(), I2C::Error> {
        // Configure all pins as inputs (default)
        self.write_register(REG_CONFIG, 0xFF).await?;

        // No polarity inversion
        self.write_register(REG_POLARITY_INV, 0x00).await?;

        // Clear output register
        self.write_register(REG_OUTPUT_PORT, 0x00).await?;

        Ok(())
    }

    /// Read all 8 input pins
    pub async fn read_port(&self) -> Result<u8, I2C::Error> {
        self.read_register(REG_INPUT_PORT).await
    }

    /// Read a single input pin
    pub async fn read_pin(&self, pin: Pin) -> Result<bool, I2C::Error> {
        let value = self.read_register(REG_INPUT_PORT).await?;
        Ok((value & pin.mask()) != 0)
    }

    /// Write all 8 output pins
    pub async fn write_port(&mut self, value: u8) -> Result<(), I2C::Error> {
        self.write_register(REG_OUTPUT_PORT, value).await
    }

    /// Write a single output pin (read-modify-write)
    pub async fn write_pin(&mut self, pin: Pin, state: bool) -> Result<(), I2C::Error> {
        let mut value = self.read_register(REG_OUTPUT_PORT).await?;

        if state {
            value |= pin.mask();
        } else {
            value &= !pin.mask();
        }

        self.write_register(REG_OUTPUT_PORT, value).await
    }

    /// Set pin direction configuration
    /// - Bit = 1: pin configured as input (high-Z)
    /// - Bit = 0: pin configured as output
    pub async fn set_direction(&mut self, config: u8) -> Result<(), I2C::Error> {
        self.write_register(REG_CONFIG, config).await
    }

    /// Set individual pin direction
    /// - true: input (high-Z)
    /// - false: output
    pub async fn set_pin_direction(&mut self, pin: Pin, is_input: bool) -> Result<(), I2C::Error> {
        let mut config = self.read_register(REG_CONFIG).await?;

        if is_input {
            config |= pin.mask();
        } else {
            config &= !pin.mask();
        }

        self.write_register(REG_CONFIG, config).await
    }

    /// Set polarity inversion register
    /// - Bit = 1: corresponding input pin polarity is inverted
    /// - Bit = 0: corresponding input pin polarity is retained
    pub async fn set_polarity(&mut self, invert: u8) -> Result<(), I2C::Error> {
        self.write_register(REG_POLARITY_INV, invert).await
    }

    /// Set individual pin polarity inversion
    pub async fn set_pin_polarity(&mut self, pin: Pin, inverted: bool) -> Result<(), I2C::Error> {
        let mut polarity = self.read_register(REG_POLARITY_INV).await?;

        if inverted {
            polarity |= pin.mask();
        } else {
            polarity &= !pin.mask();
        }

        self.write_register(REG_POLARITY_INV, polarity).await
    }

    /// Read input port and clear interrupt
    /// Reading the input register clears the INT pin
    pub async fn read_and_clear_int(&self) -> Result<u8, I2C::Error> {
        self.read_register(REG_INPUT_PORT).await
    }

    /// Sync output register to current input values
    /// Useful for preventing glitches when changing pin directions
    pub async fn sync_outputs_to_inputs(&mut self) -> Result<(), I2C::Error> {
        let input = self.read_register(REG_INPUT_PORT).await?;
        self.write_register(REG_OUTPUT_PORT, input).await
    }

    async fn write_register(&self, reg: u8, value: u8) -> Result<(), I2C::Error> {
        let data = [reg, value];
        self.i2c.lock().await.write(self.address, &data).await
    }

    async fn read_register(&self, reg: u8) -> Result<u8, I2C::Error> {
        let mut lock = self.i2c.lock().await;
        lock.write(self.address, &[reg]).await?;

        let mut buffer = [0u8; 1];
        lock.read(self.address, &mut buffer).await?;

        Ok(buffer[0])
    }

    /// Helper method to set a specific pin value in the port byte
    /// Returns updated port value
    pub fn set_pin_value(&self, port: u8, pin: Pin, state: bool) -> u8 {
        let pin_mask = 1 << (pin as u8);
        if state {
            port | pin_mask
        } else {
            port & !pin_mask
        }
    }
}
