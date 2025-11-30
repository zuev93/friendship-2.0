use crate::{
    app::{events::CURRENT_CLARIFIER_MODE, types::ClarifierMode},
    front_panel::{
        tasks::spi_receiver::handle_response_packet,
        types::{ButtonFunction, ControlBusType},
    },
};
use common::protocol_types::{LedCommand, LedState};
use common::spi_protocol::Packet;

use super::find_button_id;

#[embassy_executor::task]
pub async fn rit_mode_led_task(control_bus: ControlBusType) {
    let Some(led_id) = find_button_id(ButtonFunction::Rit) else {
        return;
    };

    loop {
        let clarifier_mode = CURRENT_CLARIFIER_MODE.wait().await;

        let state = match clarifier_mode {
            ClarifierMode::Off => LedState::Off,
            ClarifierMode::Rit => LedState::Red,
            ClarifierMode::XIT => LedState::Green,
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
