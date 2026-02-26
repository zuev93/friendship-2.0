use embassy_stm32::{dac::DacCh1, mode, peripherals::*, Peri};

pub struct SMeter {
    pub dac: DacCh1<'static, DAC1, mode::Blocking>,
}

impl SMeter {
    pub fn new(dac: Peri<'static, DAC1>, pin: Peri<'static, PA4>) -> Self {
        let dac = DacCh1::new_blocking(dac, pin);
        Self { dac }
    }
}
