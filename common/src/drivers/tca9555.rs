/*
 * TCA9555 16-bit I2C I/O Expander Driver
 *
 * Features:
 * - Two 8-bit ports (Port 0 and Port 1)
 * - Configurable as inputs or outputs
 * - Polarity inversion support
 * - I2C interface up to 400 kHz
 * - Interrupt output (INT) - active low, open-drain
 *
 * Interrupt Behavior:
 * - INT pin goes LOW when input state differs from output register
 * - Reading input registers clears the interrupt
 * - Useful for event-driven input handling instead of polling
 */

use core::result::Result;
use embassy_sync::mutex::Mutex;
use crate::PlatformMutex;
use embedded_hal_async::i2c::I2c;

const REG_INPUT_PORT0: u8 = 0x00;
const REG_INPUT_PORT1: u8 = 0x01;
const REG_OUTPUT_PORT0: u8 = 0x02;
const REG_OUTPUT_PORT1: u8 = 0x03;
const REG_POLARITY_INV_PORT0: u8 = 0x04;
const REG_POLARITY_INV_PORT1: u8 = 0x05;
const REG_CONFIG_PORT0: u8 = 0x06;
const REG_CONFIG_PORT1: u8 = 0x07;

#[derive(Clone, Copy, Debug)]
pub enum Port {
    Port0,
    Port1,
}

#[derive(Clone, Copy, Debug)]
pub enum Pin {
    Pin0,
    Pin1,
    Pin2,
    Pin3,
    Pin4,
    Pin5,
    Pin6,
    Pin7,
}

pub struct TCA9555<I2C>
where
    I2C: I2c + 'static,
{
    address: u8,
    i2c: &'static Mutex<PlatformMutex, I2C>,
}

impl<I2C> TCA9555<I2C>
where
    I2C: I2c + 'static,
{
    pub fn new(i2c_addr: u8, i2c: &'static Mutex<PlatformMutex, I2C>) -> Self {
        Self {
            address: i2c_addr,
            i2c,
        }
    }

    /// Set a specific pin value in port values
    /// Returns updated (port0, port1) values
    pub fn set_pin_value(
        &self,
        port0: u8,
        port1: u8,
        port: Port,
        pin: Pin,
        state: bool,
    ) -> (u8, u8) {
        let pin_mask = 1 << (pin as u8);

        match port {
            Port::Port0 => {
                let new_port0 = if state {
                    port0 | pin_mask
                } else {
                    port0 & !pin_mask
                };
                (new_port0, port1)
            }
            Port::Port1 => {
                let new_port1 = if state {
                    port1 | pin_mask
                } else {
                    port1 & !pin_mask
                };
                (port0, new_port1)
            }
        }
    }

    pub async fn init(&mut self) -> Result<(), I2C::Error> {
        self.configure_port(Port::Port0, 0xFF).await?;
        self.configure_port(Port::Port1, 0xFF).await?;

        self.set_polarity(Port::Port0, 0x00).await?;
        self.set_polarity(Port::Port1, 0x00).await?;

        self.write_port(Port::Port0, 0x00).await?;
        self.write_port(Port::Port1, 0x00).await?;

        Ok(())
    }

    pub async fn configure_port(&mut self, port: Port, config: u8) -> Result<(), I2C::Error> {
        let reg = match port {
            Port::Port0 => REG_CONFIG_PORT0,
            Port::Port1 => REG_CONFIG_PORT1,
        };
        self.write_register(reg, config).await
    }

    pub async fn write_port(&mut self, port: Port, value: u8) -> Result<(), I2C::Error> {
        let reg = match port {
            Port::Port0 => REG_OUTPUT_PORT0,
            Port::Port1 => REG_OUTPUT_PORT1,
        };
        self.write_register(reg, value).await
    }

    pub async fn write_pin(&mut self, port: Port, pin: Pin, state: bool) -> Result<(), I2C::Error> {
        let reg = match port {
            Port::Port0 => REG_OUTPUT_PORT0,
            Port::Port1 => REG_OUTPUT_PORT1,
        };

        let mut value = self.read_register(reg).await?;

        if state {
            value |= 1 << pin as u8;
        } else {
            value &= !(1 << pin as u8);
        }

        self.write_register(reg, value).await
    }

    pub async fn set_polarity(&mut self, port: Port, invert: u8) -> Result<(), I2C::Error> {
        let reg = match port {
            Port::Port0 => REG_POLARITY_INV_PORT0,
            Port::Port1 => REG_POLARITY_INV_PORT1,
        };
        self.write_register(reg, invert).await
    }

    pub async fn set_port_direction(
        &mut self,
        port: Port,
        direction: u8,
    ) -> Result<(), I2C::Error> {
        self.configure_port(port, direction).await
    }

    pub async fn set_port_polarity(&mut self, port: Port, invert: u8) -> Result<(), I2C::Error> {
        self.set_polarity(port, invert).await
    }

    pub async fn read_and_clear_int(&self) -> Result<(u8, u8), I2C::Error> {
        let port0 = self.read_register(REG_INPUT_PORT0).await?;
        let port1 = self.read_register(REG_INPUT_PORT1).await?;
        Ok((port0, port1))
    }

    pub async fn sync_outputs_to_inputs(&mut self) -> Result<(), I2C::Error> {
        let port0 = self.read_register(REG_INPUT_PORT0).await?;
        let port1 = self.read_register(REG_INPUT_PORT1).await?;

        self.write_register(REG_OUTPUT_PORT0, port0).await?;
        self.write_register(REG_OUTPUT_PORT1, port1).await?;

        Ok(())
    }

    async fn write_register(&self, reg: u8, value: u8) -> Result<(), I2C::Error> {
        let data = [reg, value];
        let mut lock = self.i2c.lock().await;
        lock.write(self.address, &data).await
    }

    async fn read_register(&self, reg: u8) -> Result<u8, I2C::Error> {
        let mut lock = self.i2c.lock().await;
        lock.write(self.address, &[reg]).await?;

        let mut buffer = [0u8; 1];
        lock.read(self.address, &mut buffer).await?;

        Ok(buffer[0])
    }
}
