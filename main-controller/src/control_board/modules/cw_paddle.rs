use embassy_stm32::gpio::{Input, Pin, Pull};
use embassy_stm32::Peri;

pub struct CwPaddle {
    dit: Input<'static>,
    dah: Input<'static>,
}

impl CwPaddle {
    pub fn new(dit_pin: Peri<'static, impl Pin>, dah_pin: Peri<'static, impl Pin>) -> Self {
        Self {
            dit: Input::new(dit_pin, Pull::Up),
            dah: Input::new(dah_pin, Pull::Up),
        }
    }

    pub fn dit_pressed(&self) -> bool {
        self.dit.is_low()
    }

    pub fn dah_pressed(&self) -> bool {
        self.dah.is_low()
    }
}
