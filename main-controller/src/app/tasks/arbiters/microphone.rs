use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::events::MICROPHONE;
use crate::app::types::Microphone;

pub static MICROPHONE_CMD: Signal<ThreadModeRawMutex, i16> = Signal::new();

#[embassy_executor::task]
pub async fn microphone_arbiter_task() {
    let mut microphone = Microphone::new(0);

    loop {
        let delta = MICROPHONE_CMD.wait().await;
        let new_val = Microphone::new(microphone.raw() + delta);
        if new_val != microphone {
            microphone = new_val;
            MICROPHONE.sender().send(microphone);
        }
    }
}
