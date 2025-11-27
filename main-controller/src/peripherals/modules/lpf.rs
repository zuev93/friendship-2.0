/*
 * LPF (Low Pass Filter) Module
 *
 * Controls output low-pass filters for different frequency bands
 * Uses PCA9534 GPIO expander for relay control
 */

use crate::app::types::{Frequency, Mode};
use crate::peripherals::config::LpfConfig;
use crate::peripherals::types::{PeripherialI2c, PeripherialI2cMutex};
use common::drivers::pca9534::PCA9534;

const LPF_GPIO_ADDR: u8 = 0x20; // TODO: Move to i2c_map

pub struct Lpf {
    gpio: PCA9534<PeripherialI2c>,
    lpf_config: LpfConfig,
    mode: Mode,
    frequency: Frequency,
}

impl Lpf {
    pub fn new(i2c: PeripherialI2cMutex, lpf_config: LpfConfig) -> Self {
        Self {
            gpio: PCA9534::new(LPF_GPIO_ADDR, i2c),
            lpf_config,
            mode: Mode::StandBy,
            frequency: 0,
        }
    }

    pub async fn set_mode(&mut self, mode: Mode) -> Result<(), &'static str> {
        self.mode = mode;
        self.update_state().await
    }

    pub async fn set_frequency(&mut self, frequency: Frequency) -> Result<(), &'static str> {
        self.frequency = frequency;
        self.update_state().await
    }

    pub async fn update_state(&mut self) -> Result<(), &'static str> {
        let mut port_value = 0x00;

        if self.mode == Mode::StandBy {
            self.gpio
                .init()
                .await
                .map_err(|_| "Failed to initialize LPF GPIO")?;

            self.gpio
                .set_direction(0x00)
                .await
                .map_err(|_| "Failed to set LPF GPIO direction")?;
        }

        port_value = self.gpio.set_pin_value(
            port_value,
            self.lpf_config.control.tx_pin,
            self.mode == Mode::Tx,
        );
        port_value = self.gpio.set_pin_value(
            port_value,
            self.lpf_config.find_filter(self.frequency),
            self.mode == Mode::Rx || self.mode == Mode::Tx,
        );

        self.gpio
            .write_port(port_value)
            .await
            .map_err(|_| "Failed to write LPF port")?;

        Ok(())
    }
}
