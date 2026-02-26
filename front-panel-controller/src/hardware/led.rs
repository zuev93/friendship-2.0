use embassy_stm32::{
    gpio::{Level, Output, Pin, Speed},
    Peri,
};

pub struct Led {
    pub pin_x: Output<'static>,
    pub pin_y: Output<'static>,
}

impl Led {
    pub fn new(
        pin_x: Peri<'static, impl Pin>,
        pin_y: Peri<'static, impl Pin>,
    ) -> Self {
        Self {
            pin_x: Output::new(pin_x, Level::Low, Speed::Medium),
            pin_y: Output::new(pin_y, Level::Low, Speed::Medium),
        }
    }
}

pub struct Leds {
    pub leds: [Led; 7],
}
