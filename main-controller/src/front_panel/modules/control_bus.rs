use common::spi_protocol::{Packet, PacketType};
use embassy_stm32::{
    gpio::{Level, Output, Pin, Speed},
    mode,
    spi::{self, MisoPin, MosiPin, RxDma, SckPin, Spi, TxDma},
    time::Hertz,
    Peri,
};

pub struct ControlBus {
    spi: Spi<'static, mode::Async, spi::mode::Master>,
    cs: Output<'static>,
    idle_packet: Packet,
}

impl ControlBus {
    pub fn new<T: spi::Instance>(
        spi_bus: Peri<'static, T>,
        bus_mosi: Peri<'static, impl MosiPin<T>>,
        bus_miso: Peri<'static, impl MisoPin<T>>,
        bus_sck: Peri<'static, impl SckPin<T>>,
        bus_dma_tx: Peri<'static, impl TxDma<T>>,
        bus_dma_rx: Peri<'static, impl RxDma<T>>,
        bus_cs_pin: Peri<'static, impl Pin>,
    ) -> Self {
        let mut spi_config = spi::Config::default();
        spi_config.frequency = Hertz(10_000_000);

        let spi = Spi::new(
            spi_bus, bus_sck, bus_mosi, bus_miso, bus_dma_tx, bus_dma_rx, spi_config,
        );
        let cs = Output::new(bus_cs_pin, Level::High, Speed::High);

        let mut idle_packet = Packet::new();
        idle_packet.set_type(PacketType::Idle);
        idle_packet.set_crc();

        Self {
            spi,
            cs,
            idle_packet,
        }
    }

    pub async fn exchange(&mut self, tx_packet: &Packet) -> Result<Packet, ()> {
        let mut rx_packet = Packet::new();

        self.cs.set_low();

        let result: Result<(), _> = self
            .spi
            .transfer::<u8>(&mut rx_packet.data, &tx_packet.data)
            .await
            .map_err(|_| ());

        self.cs.set_high();

        result?;

        if rx_packet.verify_crc() {
            Ok(rx_packet)
        } else {
            Err(())
        }
    }

    pub async fn send_packet(&mut self, packet: &Packet) -> Result<Packet, ()> {
        self.exchange(packet).await
    }

    pub async fn receive_packet(&mut self) -> Result<Packet, ()> {
        let idle = self.idle_packet;
        self.exchange(&idle).await
    }
}
