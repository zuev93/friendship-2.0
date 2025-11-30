use crate::app::{events::CURRENT_MODE, types::Mode};
use crate::front_panel::tasks::spi_receiver::handle_response_packet;
use crate::front_panel::types::ControlBusType;
use common::protocol_types::{LedCommand, LedState};
use common::spi_protocol::Packet;

#[embassy_executor::task]
pub async fn mode_led_task(control_bus: ControlBusType) {
    const LED_ID: u8 = 0; // Power/Mode button

    loop {
        let mode = CURRENT_MODE.wait().await;

        let state = match mode {
            Mode::StandBy => LedState::Red,
            Mode::WarmUp | Mode::Rx | Mode::Tx => LedState::Green,
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
