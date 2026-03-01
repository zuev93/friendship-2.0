use crate::{
    app::events::TONE,
    front_panel::{
        tasks::spi_receiver::handle_response_packet,
        types::{ButtonFunction, ControlBusType},
    },
};
use common::protocol_types::{LedCommand, LedState};

use crate::runtime_stats::TaskId;
use druzhba_macros::instrumented;

use super::find_button_id;

#[instrumented(TaskId::ToneLed)]
#[embassy_executor::task]
pub async fn tone_led_task(control_bus: ControlBusType) {
    let Some(led_id) = find_button_id(ButtonFunction::Tone) else {
        return;
    };

    let mut tone_rcv = TONE.receiver().unwrap();
    loop {
        let tone_active = tone_rcv.changed().await;

        let state = if tone_active {
            LedState::Green
        } else {
            LedState::Off
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
