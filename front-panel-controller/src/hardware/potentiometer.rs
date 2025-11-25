use embassy_stm32::{adc::Adc, peripherals::*};

pub struct Potentiometers {
    pub adc1: Adc<'static, ADC1>,
    pub var1: PA0,
    pub var2: PA1,
    pub var3: PA2,
    pub var4: PA3,
    pub var5: PA7,
    pub var6: PB0,
}

impl Potentiometers {
    pub fn new(
        adc1: ADC1,
        var1: PA0,
        var2: PA1,
        var3: PA2,
        var4: PA3,
        var5: PA7,
        var6: PB0,
    ) -> Self {
        let mut delay = embassy_time::Delay;
        let adc1 = Adc::new(adc1, &mut delay);

        Self {
            adc1,
            var1,
            var2,
            var3,
            var4,
            var5,
            var6,
        }
    }
}
