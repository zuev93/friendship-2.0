use crate::{
    app::{events::TRANSMIT_MODE, types::TransmitMode},
    front_panel::{
        tasks::spi_receiver::handle_response_packet,
        types::{ButtonFunction, ControlBusType},
    },
};
use common::protocol_types::{LedCommand, LedState};

use super::find_button_id;

#[embassy_executor::task]
pub async fn transmit_mode_led_task(control_bus: ControlBusType) {
    let Some(led_id) = find_button_id(ButtonFunction::TransmitMode) else {
        return;
    };

    loop {
        let transmit_mode = TRANSMIT_MODE.wait().await;

        let state = match transmit_mode {
            TransmitMode::Usb => LedState::Green,
            TransmitMode::Lsb => LedState::Green,
            TransmitMode::Cw => LedState::Red,
            TransmitMode::Am => LedState::Off,
        };

        let led_cmd = LedCommand { led_id, state };

        let response = {
            let mut spi = control_bus.lock().await;
            spi.send(&led_cmd).await
        };

        if let Ok(response_packet) = response {
            handle_response_packet(&response_packet).await;
        }
    }
}
