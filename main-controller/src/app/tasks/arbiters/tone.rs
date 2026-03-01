use druzhba_macros::instrumented;
use crate::runtime_stats::TaskId;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::events::TONE;

pub enum ToneCommand {
    Press,
    Release,
}

pub static TONE_CMD: Signal<ThreadModeRawMutex, ToneCommand> = Signal::new();

#[instrumented(TaskId::ToneArbiter)]
#[embassy_executor::task]
pub async fn tone_arbiter_task() {
    loop {
        let cmd = TONE_CMD.wait().await;
        let active = match cmd {
            ToneCommand::Press => true,
            ToneCommand::Release => false,
        };
        TONE.sender().send(active);
    }
}
