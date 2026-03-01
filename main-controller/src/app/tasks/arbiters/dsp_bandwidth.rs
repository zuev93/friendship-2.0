use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::events::DSP_BW;
use crate::app::types::DspBandwidth;

pub static DSP_BANDWIDTH_CMD: Signal<ThreadModeRawMutex, i16> = Signal::new();

#[embassy_executor::task]
pub async fn dsp_bandwidth_arbiter_task() {
    let mut bw = DspBandwidth::new(2700);

    loop {
        let delta = DSP_BANDWIDTH_CMD.wait().await;
        let new_val = bw.add(delta);
        if new_val != bw {
            bw = new_val;
            DSP_BW.sender().send(bw);
        }
    }
}
