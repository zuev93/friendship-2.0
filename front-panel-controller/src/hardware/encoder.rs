use embassy_stm32::exti::Channel;
use embassy_stm32::gpio::Pin;
use embassy_stm32::{
    exti::ExtiInput,
    gpio::{AnyPin, Input, Pull},
    Peripheral,
};

pub struct Encoder {
    pub channel_a: ExtiInput<'static, AnyPin>,
    pub channel_b: ExtiInput<'static, AnyPin>,
}

impl Encoder {
    pub fn new<'d, T1: Pin, T2: Pin>(
        pin1: impl Peripheral<P = T1> + 'static,
        pin2: impl Peripheral<P = T2> + 'static,
        ch1: impl Peripheral<P = T1::ExtiChannel> + Channel + 'static,
        ch2: impl Peripheral<P = T2::ExtiChannel> + Channel + 'static,
    ) -> Option<Encoder> {
        Some(Encoder {
            channel_a: ExtiInput::new(Input::new(pin1, Pull::Up).degrade(), ch1.degrade()),
            channel_b: ExtiInput::new(Input::new(pin2, Pull::Up).degrade(), ch2.degrade()),
        })
    }
}

pub struct Encoders {
    pub encoders: [Option<Encoder>; 2],
}
