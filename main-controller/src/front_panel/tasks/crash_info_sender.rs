use crate::crash_info::LAST_CRASH;
use crate::front_panel::tasks::spi_receiver::handle_response_packet;
use crate::front_panel::types::ControlBusType;
use common::protocol_types::CrashInfoCommand;

#[embassy_executor::task]
pub async fn crash_info_sender_task(control_bus: ControlBusType) {
    let mut rcv = LAST_CRASH.receiver().unwrap();
    let crash = rcv.changed().await;

    let cmd = CrashInfoCommand {
        reset_reason: crash.reset_reason,
        pc: crash.pc,
        lr: crash.lr,
        panic_line: crash.panic_line,
        panic_file: crash.panic_file,
        uptime_secs: crash.uptime_secs,
    };

    let response = {
        let mut spi = control_bus.lock().await;
        spi.send(&cmd).await
    };

    if let Ok(response_packet) = response {
        handle_response_packet(&response_packet).await;
    }
}
