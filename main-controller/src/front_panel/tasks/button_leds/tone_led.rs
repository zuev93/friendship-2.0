use crate::{
    app::events::TONE_ACTIVE,
    front_panel::{
        tasks::spi_receiver::handle_response_packet,
        types::{ButtonFunction, ControlBusType},
    },
};
use common::protocol_types::{LedCommand, LedState};
use common::spi_protocol::Packet;

use super::find_button_id;

#[embassy_executor::task]
pub async fn tone_led_task(control_bus: ControlBusType) {
    let Some(led_id) = find_button_id(ButtonFunction::Tone) else {
        return;
    };

    loop {
        let tone_active = TONE_ACTIVE.wait().await;

        let state = if tone_active {
            LedState::Green
        } else {
            LedState::Off
        };

        let led_cmd = LedCommand { led_id, state };
        let mut packet = Packet::new();
        led_cmd.serialize(&mut packet);

        let response = {
            let mut spi = control_bus.lock().await;
            spi.send_packet(&packet).await
        };

        if let Ok(response_packet) = response {
            handle_response_packet(&response_packet).await;
        }
    }
}
