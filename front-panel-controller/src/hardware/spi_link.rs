use embassy_stm32::{
    gpio::{Level, Output, Pin, Speed},
    mode,
    peripherals::*,
    spi::{self, CsPin, MisoPin, MosiPin, SckPin, Spi},
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
        sck: Peri<'static, impl SckPin<SPI1>>,
        mosi: Peri<'static, impl MosiPin<SPI1>>,
        miso: Peri<'static, impl MisoPin<SPI1>>,
        nss: Peri<'static, impl CsPin<SPI1>>,
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
