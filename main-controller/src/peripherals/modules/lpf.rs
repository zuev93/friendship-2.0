/*
 * LPF (Low Pass Filter) Module
 *
 * Controls output low-pass filters for different frequency bands
 * Uses PCA9534 GPIO expander for relay control
 */

use crate::app::types::{Frequency, Mode};
use crate::peripherals::config::LpfConfig;
use crate::peripherals::types::PeripherialI2c;
use common::drivers::pca9534::PCA9534;

const LPF_GPIO_ADDR: u8 = 0x20; // TODO: Move to i2c_map

pub struct Lpf {
    i2c: PeripherialI2c,
    gpio: PCA9534,
    lpf_config: LpfConfig,
    mode: Mode,
    frequency: Frequency,
}

impl Lpf {
    pub fn new(i2c: PeripherialI2c, lpf_config: LpfConfig) -> Self {
        Self {
            i2c,
            gpio: PCA9534::new(LPF_GPIO_ADDR),
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
        let mut i2c_guard = self.i2c.lock().await;
        let mut port_value = 0x00;

        if self.mode == Mode::StandBy {
            self.gpio
                .init(&mut *i2c_guard)
                .await
                .map_err(|_| "Failed to initialize LPF GPIO")?;

            self.gpio
                .set_direction(&mut *i2c_guard, 0x00)
                .await
                .map_err(|_| "Failed to set LPF GPIO direction")?;
        }

        port_value = PCA9534::set_pin_value(
            port_value,
            self.lpf_config.control.tx_pin,
            self.mode == Mode::Tx,
        );
        port_value = PCA9534::set_pin_value(
            port_value,
            self.lpf_config.find_filter(self.frequency),
            self.mode == Mode::Rx || self.mode == Mode::Tx,
        );

        self.gpio
            .write_port(&mut *i2c_guard, port_value)
            .await
            .map_err(|_| "Failed to write LPF port")?;

        Ok(())
    }
}
