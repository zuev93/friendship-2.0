use crate::app::{events::CURRENT_IF_GAIN_MODE, types::IfGainMode};
use crate::front_panel::tasks::spi_receiver::handle_response_packet;
use crate::front_panel::types::ControlBusType;
use common::protocol_types::{LedCommand, LedState};
use common::spi_protocol::Packet;

#[embassy_executor::task]
pub async fn agc_mode_led_task(control_bus: ControlBusType) {
    const LED_ID: u8 = 6; // AGC button

    loop {
        let agc_mode = CURRENT_IF_GAIN_MODE.wait().await;

        let state = match agc_mode {
            IfGainMode::Manual => LedState::Off,
            IfGainMode::AgcFast => LedState::Green,
            IfGainMode::AgcSlow => LedState::Red,
        };

        let led_cmd = LedCommand {
            led_id: LED_ID,
            state,
        };
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
