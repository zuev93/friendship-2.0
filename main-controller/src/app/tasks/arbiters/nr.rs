use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::events::NR_ENABLED;

pub enum NrCommand {
    Toggle,
}

pub static NR_CMD: Signal<ThreadModeRawMutex, NrCommand> = Signal::new();

#[embassy_executor::task]
pub async fn nr_arbiter_task() {
    let mut nr_enabled = false;

    loop {
        let cmd = NR_CMD.wait().await;
        match cmd {
            NrCommand::Toggle => {
                nr_enabled = !nr_enabled;
                NR_ENABLED.sender().send(nr_enabled);
            }
        }
    }
}
