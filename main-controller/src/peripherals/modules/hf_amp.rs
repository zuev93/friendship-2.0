/*
 * HF Amplifier Module
 *
 * Controls high-frequency pre-amplifier for receive path
 * Uses PCA9534 GPIO expander for amplifier control
 */

use crate::app::types::Mode;
use crate::peripherals::types::{PeripherialI2c, PeripherialI2cMutex};
use common::drivers::pca9534::PCA9534;

const HF_AMP_GPIO_ADDR: u8 = 0x22; // TODO: Move to i2c_map

pub struct HfAmp {
    #[allow(dead_code)]
    gpio: PCA9534<PeripherialI2c>,
    mode: Mode,
}

impl HfAmp {
    pub fn new(i2c: PeripherialI2cMutex) -> Self {
        Self {
            gpio: PCA9534::new(HF_AMP_GPIO_ADDR, i2c),
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
