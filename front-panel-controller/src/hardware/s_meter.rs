use embassy_stm32::{dac::DacCh1, dma::NoDma, peripherals::*};

pub struct SMeter {
    pub dac: DacCh1<'static, DAC, NoDma>,
}

impl SMeter {
    pub fn new(dac: DAC, pin: PA4) -> Self {
        let dac = DacCh1::new(dac, NoDma, pin);
        Self { dac }
    }
}
