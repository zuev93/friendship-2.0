/*
 * HF Amplifier Module
 *
 * Controls high-frequency pre-amplifier for receive path
 * Uses PCA9534 GPIO expander for amplifier control
 */

use crate::app::types::Mode;
use crate::i2c_map::I2cAddress;
use crate::peripherals::types::{PeripherialI2c, PeripherialI2cMutex};
use common::drivers::pca9534::PCA9534;

pub struct HfAmp {
    #[allow(dead_code)]
    gpio: PCA9534<PeripherialI2c>,
    mode: Mode,
}

impl HfAmp {
    pub fn new(i2c: PeripherialI2cMutex, gpio_addr: I2cAddress) -> Self {
        Self {
            gpio: PCA9534::new(gpio_addr.into(), i2c),
            mode: Mode::StandBy,
        }
    }

    pub async fn set_mode(&mut self, mode: Mode) -> Result<(), &'static str> {
        self.mode = mode;
        self.update_state().await
    }

    pub async fn update_state(&mut self) -> Result<(), &'static str> {
        // TODO implement
        Ok(())
    }
}
