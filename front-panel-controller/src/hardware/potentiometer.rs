use embassy_stm32::{adc::Adc, peripherals::*, Peri};

pub struct Potentiometers {
    pub adc1: Adc<'static, ADC1>,
    pub var1: Peri<'static, PA0>,
    pub var2: Peri<'static, PA1>,
    pub var3: Peri<'static, PA2>,
    pub var4: Peri<'static, PA3>,
    pub var5: Peri<'static, PA7>,
    pub var6: Peri<'static, PB0>,
}

impl Potentiometers {
    pub fn new(
        adc1: Peri<'static, ADC1>,
        var1: Peri<'static, PA0>,
        var2: Peri<'static, PA1>,
        var3: Peri<'static, PA2>,
        var4: Peri<'static, PA3>,
        var5: Peri<'static, PA7>,
        var6: Peri<'static, PB0>,
    ) -> Self {
        let adc1 = Adc::new(adc1);

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
