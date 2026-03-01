use druzhba_macros::instrumented;
use crate::runtime_stats::TaskId;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::events::AUDIO_AGC_MODE;
use crate::app::types::AudioAgcMode;

pub enum AudioAgcCommand {
    Cycle,
}

pub static AUDIO_AGC_CMD: Signal<ThreadModeRawMutex, AudioAgcCommand> = Signal::new();

#[instrumented(TaskId::AudioAgcArbiter)]
#[embassy_executor::task]
pub async fn audio_agc_arbiter_task() {
    let mut mode = AudioAgcMode::Off;

    loop {
        let cmd = AUDIO_AGC_CMD.wait().await;
        match cmd {
            AudioAgcCommand::Cycle => {
                mode = mode.next();
                AUDIO_AGC_MODE.sender().send(mode);
            }
        }
    }
}
