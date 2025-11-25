/*
 * HF Amplifier Module
 *
 * Controls high-frequency pre-amplifier for receive path
 * Uses PCA9534 GPIO expander for amplifier control
 */

use crate::app::types::Mode;
use crate::peripherals::types::PeripherialI2c;
use common::drivers::pca9534::PCA9534;

const HF_AMP_GPIO_ADDR: u8 = 0x22; // TODO: Move to i2c_map

pub struct HfAmp {
    i2c: PeripherialI2c,
    #[allow(dead_code)]
    gpio: PCA9534,
    mode: Mode,
}

impl HfAmp {
    pub fn new(i2c: PeripherialI2c) -> Self {
        Self {
            i2c,
            gpio: PCA9534::new(HF_AMP_GPIO_ADDR),
            mode: Mode::StandBy,
        }
    }

    pub async fn set_mode(&mut self, mode: Mode) -> Result<(), &'static str> {
        self.mode = mode;
        self.update_state().await
    }

    pub async fn update_state(&mut self) -> Result<(), &'static str> {
        let mut _i2c_guard = self.i2c.lock().await;
        // TODO implement
        Ok(())
    }
}
