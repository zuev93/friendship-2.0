use common::protocol_types::MenuCommand;
use embassy_futures::select::{select, Either};

use crate::front_panel::{
    events::{MENU_CMD_QUEUE, MENU_ENCODER_EVENTS},
    tasks::spi_receiver::handle_response_packet,
    types::ControlBusType,
};
use crate::runtime_stats::TaskId;
use druzhba_macros::instrumented;

#[instrumented(TaskId::MenuSender)]
#[embassy_executor::task]
pub async fn menu_sender_task(control_bus: ControlBusType) {
    loop {
        let cmd = match select(MENU_CMD_QUEUE.receive(), MENU_ENCODER_EVENTS.receive()).await {
            Either::First(cmd) => cmd,
            Either::Second(delta) => MenuCommand::Scroll(delta),
        };

        let response = {
            let mut spi = control_bus.lock().await;
            spi.send(&cmd).await
        };

        if let Ok(response_packet) = response {
            handle_response_packet(&response_packet).await;
        }
    }
}
