use embassy_stm32::gpio::Pin;
use embassy_stm32::{
    gpio::{AnyPin, Level, Output, Speed},
    Peripheral,
};

pub struct Led {
    pub pin_x: Output<'static, AnyPin>,
    pub pin_y: Output<'static, AnyPin>,
}

impl Led {
    pub fn new(
        pin_x: impl Peripheral<P = impl Pin> + 'static,
        pin_y: impl Peripheral<P = impl Pin> + 'static,
    ) -> Self {
        Self {
            pin_x: Output::new(pin_x, Level::Low, Speed::Medium).degrade(),
            pin_y: Output::new(pin_y, Level::Low, Speed::Medium).degrade(),
        }
    }
}

pub struct Leds {
    pub leds: [Led; 7],
}
