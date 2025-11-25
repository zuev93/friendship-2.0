use embassy_stm32::{
    gpio::{AnyPin, Level, Output, Pin, Speed},
    peripherals::*,
    Peripheral,
};

use super::spi_slave::SpiSlave;

pub struct SpiLink {
    pub spi: SpiSlave,
    pub link_alert: Output<'static, AnyPin>,
}

impl SpiLink {
    pub fn new(
        spi1: SPI1,
        sck: PA5,
        mosi: PB5,
        miso: PA6,
        nss: PA15,
        link_alert_pin: impl Peripheral<P = impl Pin> + 'static,
    ) -> Self {
        let spi = SpiSlave::new(spi1, sck, mosi, miso, nss);

        Self {
            spi,
            link_alert: Output::new(link_alert_pin, Level::Low, Speed::High).degrade(),
        }
    }
}
