use druzhba_macros::instrumented;
use crate::runtime_stats::TaskId;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::events::CLARIFIER_MODE;
use crate::app::types::ClarifierMode;

pub enum ClarifierModeCommand {
    Toggle,
}

pub static CLARIFIER_MODE_CMD: Signal<ThreadModeRawMutex, ClarifierModeCommand> = Signal::new();

#[instrumented(TaskId::ClarifierModeArbiter)]
#[embassy_executor::task]
pub async fn clarifier_mode_arbiter_task() {
    let mut clarifier_mode = ClarifierMode::Off;

    loop {
        let cmd = CLARIFIER_MODE_CMD.wait().await;
        match cmd {
            ClarifierModeCommand::Toggle => {
                clarifier_mode = clarifier_mode.toggle();
                CLARIFIER_MODE.sender().send(clarifier_mode);
            }
        }
    }
}
