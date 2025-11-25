use crate::{
    app::{events::CURRENT_TRANSMIT_MODE, types::TransmitMode},
    front_panel::{
        tasks::spi_receiver::handle_response_packet,
        types::{ButtonFunction, SpiType},
    },
};
use common::protocol_types::{LedCommand, LedState};
use common::spi_protocol::Packet;

use super::find_button_id;

#[embassy_executor::task]
pub async fn transmit_mode_led_task(spi_link: SpiType) {
    let Some(led_id) = find_button_id(ButtonFunction::TransmitMode) else {
        return;
    };

    loop {
        let transmit_mode = CURRENT_TRANSMIT_MODE.wait().await;

        let state = match transmit_mode {
            TransmitMode::Usb => LedState::Green,
            TransmitMode::Lsb => LedState::Green,
            TransmitMode::Cw => LedState::Red,
            TransmitMode::Am => LedState::Off,
        };

        let led_cmd = LedCommand { led_id, state };
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
