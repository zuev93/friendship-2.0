use druzhba_macros::instrumented;
use crate::runtime_stats::TaskId;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::events::CW_PEAK_WIDTH;
use crate::app::types::CwPeakWidth;

pub static CW_PEAK_WIDTH_CMD: Signal<ThreadModeRawMutex, i16> = Signal::new();

#[instrumented(TaskId::CwPeakWidthArbiter)]
#[embassy_executor::task]
pub async fn cw_peak_width_arbiter_task() {
    let mut width = CwPeakWidth::new(200);

    loop {
        let delta = CW_PEAK_WIDTH_CMD.wait().await;
        let new_val = width.add(delta);
        if new_val != width {
            width = new_val;
            CW_PEAK_WIDTH.sender().send(width);
        }
    }
}
