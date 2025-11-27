use crate::{
    app::{events::CURRENT_RF_GAIN_MODE, types::RfGainMode},
    front_panel::{
        tasks::spi_receiver::handle_response_packet,
        types::{ButtonFunction, SpiType},
    },
};
use common::protocol_types::{LedCommand, LedState};
use common::spi_protocol::Packet;

use super::find_button_id;

#[embassy_executor::task]
pub async fn rf_gain_mode_led_task(spi_link: SpiType) {
    let Some(led_id) = find_button_id(ButtonFunction::RfGain) else {
        return;
    };

    loop {
        let rf_gain_mode = CURRENT_RF_GAIN_MODE.wait().await;

        let state = match rf_gain_mode {
            RfGainMode::Attenuator => LedState::Red,
            RfGainMode::Normal => LedState::Off,
            RfGainMode::RfSingle | RfGainMode::RfDouble => LedState::Green,
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
