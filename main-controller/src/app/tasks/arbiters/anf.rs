use druzhba_macros::instrumented;
use crate::runtime_stats::TaskId;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::events::ANF_ENABLED;

pub enum AnfCommand {
    Toggle,
}

pub static ANF_CMD: Signal<ThreadModeRawMutex, AnfCommand> = Signal::new();

#[instrumented(TaskId::AnfArbiter)]
#[embassy_executor::task]
pub async fn anf_arbiter_task() {
    let mut anf_enabled = false;

    loop {
        let cmd = ANF_CMD.wait().await;
        match cmd {
            AnfCommand::Toggle => {
                anf_enabled = !anf_enabled;
                ANF_ENABLED.sender().send(anf_enabled);
            }
        }
    }
}
