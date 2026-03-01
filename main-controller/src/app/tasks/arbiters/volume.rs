use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::events::VOLUME;
use crate::app::types::Volume;

pub static VOLUME_CMD: Signal<ThreadModeRawMutex, i16> = Signal::new();

#[embassy_executor::task]
pub async fn volume_arbiter_task() {
    let mut volume = Volume::new(0);

    loop {
        let delta = VOLUME_CMD.wait().await;
        let new_val = Volume::new(volume.raw() + delta);
        if new_val != volume {
            volume = new_val;
            VOLUME.sender().send(volume);
        }
    }
}
