use embassy_stm32::exti::ExtiInput;

pub struct Button {
    pub pin: ExtiInput<'static>,
}

impl Button {
    pub fn new(exti_input: ExtiInput<'static>) -> Option<Self> {
        Some(Self { pin: exti_input })
    }
}

pub struct Buttons {
    pub buttons: [Option<Button>; 12],
}
