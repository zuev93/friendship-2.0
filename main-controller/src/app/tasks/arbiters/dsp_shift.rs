use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::events::DSP_PBT;
use crate::app::types::DspShift;

pub static DSP_SHIFT_CMD: Signal<ThreadModeRawMutex, i16> = Signal::new();

#[embassy_executor::task]
pub async fn dsp_shift_arbiter_task() {
    let mut shift = DspShift::new(0);

    loop {
        let delta = DSP_SHIFT_CMD.wait().await;
        let new_val = shift.add(delta);
        if new_val != shift {
            shift = new_val;
            DSP_PBT.sender().send(shift);
        }
    }
}
