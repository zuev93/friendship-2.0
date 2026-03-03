use embassy_stm32::gpio::{Input, Pin, Pull};
use embassy_stm32::Peri;

pub struct PttButton {
    input: Input<'static>,
}

impl PttButton {
    pub fn new(pin: Peri<'static, impl Pin>) -> Self {
        Self {
            input: Input::new(pin, Pull::Up),
        }
    }

    pub fn pressed(&self) -> bool {
        self.input.is_low()
    }
}
