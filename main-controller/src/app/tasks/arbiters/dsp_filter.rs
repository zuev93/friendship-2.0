use druzhba_macros::instrumented;
use crate::runtime_stats::TaskId;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::events::DSP_FILTER_ENABLED;

pub enum DspFilterCommand {
    Toggle,
}

pub static DSP_FILTER_CMD: Signal<ThreadModeRawMutex, DspFilterCommand> = Signal::new();

#[instrumented(TaskId::DspFilterArbiter)]
#[embassy_executor::task]
pub async fn dsp_filter_arbiter_task() {
    let mut enabled = false;

    loop {
        let cmd = DSP_FILTER_CMD.wait().await;
        match cmd {
            DspFilterCommand::Toggle => {
                enabled = !enabled;
                DSP_FILTER_ENABLED.sender().send(enabled);
            }
        }
    }
}
