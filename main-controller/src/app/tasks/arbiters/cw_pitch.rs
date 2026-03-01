use druzhba_macros::instrumented;
use crate::runtime_stats::TaskId;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::events::CW_PITCH;
use crate::app::types::CwPitch;

pub static CW_PITCH_CMD: Signal<ThreadModeRawMutex, i16> = Signal::new();

#[instrumented(TaskId::CwPitchArbiter)]
#[embassy_executor::task]
pub async fn cw_pitch_arbiter_task() {
    let mut pitch = CwPitch::new(700);

    loop {
        let delta = CW_PITCH_CMD.wait().await;
        let new_val = pitch.add(delta);
        if new_val != pitch {
            pitch = new_val;
            CW_PITCH.sender().send(pitch);
        }
    }
}
