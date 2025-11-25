use common::spi_protocol::{Packet, PacketType};
use embassy_stm32::{gpio::Output, mode, spi::Spi};

pub struct SpiLink {
    spi: Spi<'static, mode::Async>,
    cs: Output<'static>,
    idle_packet: Packet,
}

impl SpiLink {
    pub fn new(spi: Spi<'static, mode::Async>, cs: Output<'static>) -> Self {
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

        let result = self
            .spi
            .transfer(&mut rx_packet.data, &tx_packet.data)
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
