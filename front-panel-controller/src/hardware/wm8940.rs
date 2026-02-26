use druzhba_common::drivers::wm8940::Wm8940;
use embassy_stm32::{
    bind_interrupts,
    i2c::{self, I2c, Master},
    mode,
    peripherals::*,
    Peri,
};

bind_interrupts!(pub struct Irqs {
    I2C1_EV => i2c::EventInterruptHandler<I2C1>;
    I2C1_ER => i2c::ErrorInterruptHandler<I2C1>;
});

pub fn new_wm8940(
    i2c1: Peri<'static, I2C1>,
    scl: Peri<'static, PB6>,
    sda: Peri<'static, PB7>,
    i2c_tx_dma: Peri<'static, GPDMA1_CH0>,
    i2c_rx_dma: Peri<'static, GPDMA1_CH1>,
) -> Wm8940<I2c<'static, mode::Async, Master>> {
    let mut i2c_config = i2c::Config::default();
    i2c_config.sda_pullup = true;
    i2c_config.scl_pullup = true;

    let i2c = I2c::new(
        i2c1,
        scl,
        sda,
        Irqs,
        i2c_tx_dma,
        i2c_rx_dma,
        i2c_config,
    );

    Wm8940::new(i2c)
}
