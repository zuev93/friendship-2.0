use crate::app::{events::CURRENT_MODE, types::Mode};
use crate::front_panel::tasks::spi_receiver::handle_response_packet;
use crate::front_panel::types::SpiType;
use common::protocol_types::{LedCommand, LedState};
use common::spi_protocol::Packet;

#[embassy_executor::task]
pub async fn transmit_led_task(spi_link: SpiType) {
    const LED_ID: u8 = 1; // Transmit button

    loop {
        let mode = CURRENT_MODE.wait().await;

        let state = match mode {
            Mode::Tx => LedState::Green,
            Mode::StandBy | Mode::WarmUp | Mode::Rx => LedState::Off,
        };

        let led_cmd = LedCommand {
            led_id: LED_ID,
            state,
        };
        let mut packet = Packet::new();
        led_cmd.serialize(&mut packet);

        let response = {
            let mut spi = spi_link.lock().await;
            spi.send_packet(&packet).await
        };

        if let Ok(response_packet) = response {
            handle_response_packet(&response_packet).await;
        }
    }
}
