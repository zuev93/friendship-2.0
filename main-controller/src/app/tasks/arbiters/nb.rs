use druzhba_macros::instrumented;
use crate::runtime_stats::TaskId;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::events::NB_ENABLED;

pub enum NbCommand {
    Toggle,
    Set(bool),
}

pub static NB_CMD: Signal<ThreadModeRawMutex, NbCommand> = Signal::new();

#[instrumented(TaskId::NbArbiter)]
#[embassy_executor::task]
pub async fn nb_arbiter_task() {
    let mut nb_enabled = false;

    loop {
        let cmd = NB_CMD.wait().await;
        match cmd {
            NbCommand::Toggle => {
                nb_enabled = !nb_enabled;
                NB_ENABLED.sender().send(nb_enabled);
            }
            NbCommand::Set(enabled) => {
                nb_enabled = enabled;
                NB_ENABLED.sender().send(nb_enabled);
            }
        }
    }
}
