use crate::app::{events::IF_GAIN_MODE, types::IfGainMode};
use crate::front_panel::tasks::spi_receiver::handle_response_packet;
use crate::front_panel::types::ControlBusType;
use common::protocol_types::{LedCommand, LedState};

#[embassy_executor::task]
pub async fn agc_mode_led_task(control_bus: ControlBusType) {
    const LED_ID: u8 = 6;

    let mut if_gain_mode_rcv = IF_GAIN_MODE.receiver().unwrap();
    loop {
        let agc_mode = if_gain_mode_rcv.changed().await;

        let state = match agc_mode {
            IfGainMode::Manual => LedState::Off,
            IfGainMode::AgcFast => LedState::Green,
            IfGainMode::AgcSlow => LedState::Red,
        };

        let led_cmd = LedCommand {
            led_id: LED_ID,
            state,
        };

        let response = {
            let mut spi = control_bus.lock().await;
            spi.send(&led_cmd).await
        };

        if let Ok(response_packet) = response {
            handle_response_packet(&response_packet).await;
        }
    }
}
