use druzhba_macros::instrumented;
use crate::runtime_stats::TaskId;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::events::IF_GAIN;
use crate::app::types::IfGain;

pub static IF_GAIN_CMD: Signal<ThreadModeRawMutex, i16> = Signal::new();

#[instrumented(TaskId::IfGainArbiter)]
#[embassy_executor::task]
pub async fn if_gain_arbiter_task() {
    let mut if_gain = IfGain::new(0);

    loop {
        let delta = IF_GAIN_CMD.wait().await;
        let new_val = IfGain::new(if_gain.raw() + delta);
        if new_val != if_gain {
            if_gain = new_val;
            IF_GAIN.sender().send(if_gain);
        }
    }
}
