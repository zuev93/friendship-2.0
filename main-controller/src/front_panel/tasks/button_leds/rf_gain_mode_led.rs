use crate::{
    app::{events::RF_GAIN_MODE, types::RfGainMode},
    front_panel::{
        tasks::spi_receiver::handle_response_packet,
        types::{ButtonFunction, ControlBusType},
    },
};
use common::protocol_types::{LedCommand, LedState};

use super::find_button_id;

#[embassy_executor::task]
pub async fn rf_gain_mode_led_task(control_bus: ControlBusType) {
    let Some(led_id) = find_button_id(ButtonFunction::RfGain) else {
        return;
    };

    loop {
        let rf_gain_mode = RF_GAIN_MODE.wait().await;

        let state = match rf_gain_mode {
            RfGainMode::Attenuator => LedState::Red,
            RfGainMode::Normal => LedState::Off,
            RfGainMode::RfSingle | RfGainMode::RfDouble => LedState::Green,
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
