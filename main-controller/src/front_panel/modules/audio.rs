use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;

use common::protocol_types::Wm8940Command;
use common::spi_protocol::Packet;
use embassy_sync::mutex::Mutex;

use crate::front_panel::modules::spi_link::SpiLink;

pub struct Audio {
    spi_link: &'static Mutex<ThreadModeRawMutex, SpiLink>,
}

impl Audio {
    pub fn new(spi_link: &'static Mutex<ThreadModeRawMutex, SpiLink>) -> Self {
        Self { spi_link }
    }

    pub async fn set_volume(&self, volume_percent: u8) -> Result<(), ()> {
        let volume = volume_percent.min(100);

        let wm8940_cmd = Wm8940Command {
            dac_volume_left: volume,
            dac_volume_right: volume,
            adc_volume_left: 0,
            adc_volume_right: 0,
            enable: true,
        };

        let mut packet = Packet::new();
        wm8940_cmd.serialize(&mut packet);

        let mut spi = self.spi_link.lock().await;
        spi.send_packet(&packet).await.map(|_| ())
    }
}
