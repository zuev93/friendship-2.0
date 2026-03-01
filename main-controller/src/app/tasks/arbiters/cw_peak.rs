use druzhba_macros::instrumented;
use crate::runtime_stats::TaskId;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::events::CW_PEAK_ENABLED;

pub enum CwPeakCommand {
    Toggle,
}

pub static CW_PEAK_CMD: Signal<ThreadModeRawMutex, CwPeakCommand> = Signal::new();

#[instrumented(TaskId::CwPeakArbiter)]
#[embassy_executor::task]
pub async fn cw_peak_arbiter_task() {
    let mut enabled = false;

    loop {
        let cmd = CW_PEAK_CMD.wait().await;
        match cmd {
            CwPeakCommand::Toggle => {
                enabled = !enabled;
                CW_PEAK_ENABLED.sender().send(enabled);
            }
        }
    }
}
