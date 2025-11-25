use druzhba_common::drivers::wm8940::Wm8940;
use embassy_stm32::{
    bind_interrupts,
    i2c::{self, I2c},
    peripherals::*,
    time::Hertz,
};

bind_interrupts!(pub struct Irqs {
    I2C1_EV => i2c::EventInterruptHandler<I2C1>;
    I2C1_ER => i2c::ErrorInterruptHandler<I2C1>;
});

pub fn new_wm8940(
    i2c1: I2C1,
    scl: PB6,
    sda: PB7,
    i2c_tx_dma: DMA1_CH6,
    i2c_rx_dma: DMA1_CH5,
) -> Wm8940<I2c<'static, I2C1, DMA1_CH6, DMA1_CH5>> {
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
        Hertz(100_000),
        i2c_config,
    );

    Wm8940::new(i2c)
}
