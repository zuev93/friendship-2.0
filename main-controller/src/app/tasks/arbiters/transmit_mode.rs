use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::events::TRANSMIT_MODE;
use crate::app::types::TransmitMode;

pub enum TransmitModeCommand {
    Cycle,
}

pub static TRANSMIT_MODE_CMD: Signal<ThreadModeRawMutex, TransmitModeCommand> = Signal::new();

#[embassy_executor::task]
pub async fn transmit_mode_arbiter_task() {
    let mut transmit_mode = TransmitMode::Usb;

    loop {
        let cmd = TRANSMIT_MODE_CMD.wait().await;
        match cmd {
            TransmitModeCommand::Cycle => {
                transmit_mode = transmit_mode.next();
                TRANSMIT_MODE.sender().send(transmit_mode);
            }
        }
    }
}
