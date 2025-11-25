use embassy_stm32::exti::Channel;
use embassy_stm32::gpio::Pin;
use embassy_stm32::{
    exti::ExtiInput,
    gpio::{AnyPin, Input, Pull},
    Peripheral,
};

pub struct Button {
    pub pin: ExtiInput<'static, AnyPin>,
}

impl Button {
    pub fn new<'d, T: Pin>(
        pin: impl Peripheral<P = T> + 'static,
        ch: impl Peripheral<P = T::ExtiChannel> + Channel + 'static,
    ) -> Option<Self> {
        Some(Self {
            pin: ExtiInput::new(Input::new(pin, Pull::Up).degrade(), ch.degrade()),
        })
    }
}

pub struct Buttons {
    pub buttons: [Option<Button>; 12],
}
