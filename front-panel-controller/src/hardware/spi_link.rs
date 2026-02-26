use embassy_stm32::{
    gpio::{Level, Output, Pin, Speed},
    peripherals::*,
    Peri,
};

use super::spi_slave::SpiSlave;

pub struct SpiLink {
    pub spi: SpiSlave,
    pub link_alert: Output<'static>,
}

impl SpiLink {
    pub fn new(
        spi1: Peri<'static, SPI1>,
        sck: Peri<'static, PA5>,
        mosi: Peri<'static, PB5>,
        miso: Peri<'static, PA6>,
        nss: Peri<'static, PA15>,
        link_alert_pin: Peri<'static, impl Pin>,
    ) -> Self {
        let spi = SpiSlave::new(spi1, sck, mosi, miso, nss);

        Self {
            spi,
            link_alert: Output::new(link_alert_pin, Level::Low, Speed::High),
        }
    }
}
