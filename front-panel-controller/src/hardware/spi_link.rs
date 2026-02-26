use embassy_stm32::{
    gpio::{Level, Output, Pin, Speed},
    mode,
    peripherals::*,
    spi::{self, Spi},
    Peri,
};

pub type SpiSlaveInstance = Spi<'static, mode::Async, spi::mode::Slave>;

pub struct SpiLink {
    pub spi: SpiSlaveInstance,
    pub link_alert: Output<'static>,
}

impl SpiLink {
    pub fn new(
        spi1: Peri<'static, SPI1>,
        sck: Peri<'static, PA5>,
        mosi: Peri<'static, PB5>,
        miso: Peri<'static, PA6>,
        nss: Peri<'static, PA15>,
        tx_dma: Peri<'static, GPDMA1_CH3>,
        rx_dma: Peri<'static, GPDMA1_CH4>,
        link_alert_pin: Peri<'static, impl Pin>,
    ) -> Self {
        let mut config = spi::Config::default();
        config.mode = spi::MODE_3;

        let spi = Spi::new_slave(spi1, sck, mosi, miso, nss, tx_dma, rx_dma, config);

        Self {
            spi,
            link_alert: Output::new(link_alert_pin, Level::Low, Speed::High),
        }
    }
}
