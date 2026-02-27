use common::spi_protocol::{Packet, PacketSerializable, PacketType};
use embassy_stm32::{
    gpio::{Level, Output, Pin, Speed},
    mode,
    peripherals::CRC as CRC_PERI,
    spi::{self, MisoPin, MosiPin, RxDma, SckPin, Spi, TxDma},
    time::Hertz,
    Peri,
};

use crate::crc::HardwareCrc16Modbus;

pub struct ControlBus {
    spi: Spi<'static, mode::Async, spi::mode::Master>,
    cs: Output<'static>,
    crc: HardwareCrc16Modbus,
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
        crc_peripheral: Peri<'static, CRC_PERI>,
    ) -> Self {
        let mut spi_config = spi::Config::default();
        spi_config.frequency = Hertz(10_000_000);

        let spi = Spi::new(
            spi_bus, bus_sck, bus_mosi, bus_miso, bus_dma_tx, bus_dma_rx, spi_config,
        );
        let cs = Output::new(bus_cs_pin, Level::High, Speed::High);
        let crc = HardwareCrc16Modbus::new(crc_peripheral);

        let mut idle_packet = Packet::new();
        idle_packet.set_type(PacketType::Idle);
        idle_packet.set_crc(&crc);

        Self {
            spi,
            cs,
            crc,
            idle_packet,
        }
    }

    pub async fn send<T: PacketSerializable>(&mut self, msg: &T) -> Result<Packet, ()> {
        let mut packet = Packet::new();
        msg.serialize(&mut packet, &self.crc);
        self.exchange(&packet).await
    }

    pub async fn receive_packet(&mut self) -> Result<Packet, ()> {
        let idle = self.idle_packet;
        self.exchange(&idle).await
    }

    async fn exchange(&mut self, tx_packet: &Packet) -> Result<Packet, ()> {
        let mut rx_packet = Packet::new();

        self.cs.set_low();

        let result: Result<(), _> = self
            .spi
            .transfer::<u8>(&mut rx_packet.data, &tx_packet.data)
            .await
            .map_err(|_| ());

        self.cs.set_high();

        result?;

        if rx_packet.verify_crc(&self.crc) {
            Ok(rx_packet)
        } else {
            Err(())
        }
    }
}
