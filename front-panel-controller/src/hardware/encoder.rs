use embassy_stm32::exti::ExtiInput;

pub struct Encoder {
    pub channel_a: ExtiInput<'static>,
    pub channel_b: ExtiInput<'static>,
}

impl Encoder {
    pub fn new(a: ExtiInput<'static>, b: ExtiInput<'static>) -> Option<Encoder> {
        Some(Encoder {
            channel_a: a,
            channel_b: b,
        })
    }
}

pub struct Encoders {
    pub encoders: [Option<Encoder>; 2],
}
