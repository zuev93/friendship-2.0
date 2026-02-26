use embassy_stm32::gpio::Input;

pub struct Button {
    pub pin: Input<'static>,
}

impl Button {
    pub fn new(pin: Input<'static>) -> Option<Self> {
        Some(Self { pin })
    }

    pub fn is_low(&self) -> bool {
        self.pin.is_low()
    }
}

pub struct Buttons {
    pub buttons: [Option<Button>; 15],
}
