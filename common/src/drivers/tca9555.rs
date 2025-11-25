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

pub struct TCA9555 {
    address: u8,
}

impl TCA9555 {
    pub fn new(address: u8) -> Self {
        Self { address }
    }

    /// Set a specific pin value in port values
    /// Returns updated (port0, port1) values
    pub fn set_pin_value(port0: u8, port1: u8, port: Port, pin: Pin, state: bool) -> (u8, u8) {
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

    pub async fn init<I2C: I2c>(&mut self, i2c: &mut I2C) -> Result<(), I2C::Error> {
        self.configure_port(i2c, Port::Port0, 0xFF).await?;
        self.configure_port(i2c, Port::Port1, 0xFF).await?;

        self.set_polarity(i2c, Port::Port0, 0x00).await?;
        self.set_polarity(i2c, Port::Port1, 0x00).await?;

        self.write_port(i2c, Port::Port0, 0x00).await?;
        self.write_port(i2c, Port::Port1, 0x00).await?;

        Ok(())
    }

    pub async fn configure_port<I2C: I2c>(
        &mut self,
        i2c: &mut I2C,
        port: Port,
        config: u8,
    ) -> Result<(), I2C::Error> {
        let reg = match port {
            Port::Port0 => REG_CONFIG_PORT0,
            Port::Port1 => REG_CONFIG_PORT1,
        };
        self.write_register(i2c, reg, config).await
    }

    pub async fn write_port<I2C: I2c>(
        &mut self,
        i2c: &mut I2C,
        port: Port,
        value: u8,
    ) -> Result<(), I2C::Error> {
        let reg = match port {
            Port::Port0 => REG_OUTPUT_PORT0,
            Port::Port1 => REG_OUTPUT_PORT1,
        };
        self.write_register(i2c, reg, value).await
    }

    pub async fn write_pin<I2C: I2c>(
        &mut self,
        i2c: &mut I2C,
        port: Port,
        pin: Pin,
        state: bool,
    ) -> Result<(), I2C::Error> {
        let reg = match port {
            Port::Port0 => REG_OUTPUT_PORT0,
            Port::Port1 => REG_OUTPUT_PORT1,
        };

        let mut value = self.read_register(i2c, reg).await?;

        if state {
            value |= 1 << pin as u8;
        } else {
            value &= !(1 << pin as u8);
        }

        self.write_register(i2c, reg, value).await
    }

    pub async fn set_polarity<I2C: I2c>(
        &mut self,
        i2c: &mut I2C,
        port: Port,
        invert: u8,
    ) -> Result<(), I2C::Error> {
        let reg = match port {
            Port::Port0 => REG_POLARITY_INV_PORT0,
            Port::Port1 => REG_POLARITY_INV_PORT1,
        };
        self.write_register(i2c, reg, invert).await
    }

    pub async fn set_port_direction<I2C: I2c>(
        &mut self,
        i2c: &mut I2C,
        port: Port,
        direction: u8,
    ) -> Result<(), I2C::Error> {
        self.configure_port(i2c, port, direction).await
    }

    pub async fn set_port_polarity<I2C: I2c>(
        &mut self,
        i2c: &mut I2C,
        port: Port,
        invert: u8,
    ) -> Result<(), I2C::Error> {
        self.set_polarity(i2c, port, invert).await
    }

    pub async fn read_and_clear_int<I2C: I2c>(
        &self,
        i2c: &mut I2C,
    ) -> Result<(u8, u8), I2C::Error> {
        let port0 = self.read_register(i2c, REG_INPUT_PORT0).await?;
        let port1 = self.read_register(i2c, REG_INPUT_PORT1).await?;
        Ok((port0, port1))
    }

    pub async fn sync_outputs_to_inputs<I2C: I2c>(
        &mut self,
        i2c: &mut I2C,
    ) -> Result<(), I2C::Error> {
        let port0 = self.read_register(i2c, REG_INPUT_PORT0).await?;
        let port1 = self.read_register(i2c, REG_INPUT_PORT1).await?;

        self.write_register(i2c, REG_OUTPUT_PORT0, port0).await?;
        self.write_register(i2c, REG_OUTPUT_PORT1, port1).await?;

        Ok(())
    }

    async fn write_register<I2C: I2c>(
        &self,
        i2c: &mut I2C,
        reg: u8,
        value: u8,
    ) -> Result<(), I2C::Error> {
        let data = [reg, value];
        i2c.write(self.address, &data).await
    }

    async fn read_register<I2C: I2c>(&self, i2c: &mut I2C, reg: u8) -> Result<u8, I2C::Error> {
        i2c.write(self.address, &[reg]).await?;

        let mut buffer = [0u8; 1];
        i2c.read(self.address, &mut buffer).await?;

        Ok(buffer[0])
    }
}
