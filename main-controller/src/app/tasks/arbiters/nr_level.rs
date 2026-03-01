use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::events::NR_LEVEL;
use crate::app::types::NrLevel;

pub static NR_LEVEL_CMD: Signal<ThreadModeRawMutex, i16> = Signal::new();

#[embassy_executor::task]
pub async fn nr_level_arbiter_task() {
    let mut nr_level = NrLevel::new(0);

    loop {
        let delta = NR_LEVEL_CMD.wait().await;
        let new_val = NrLevel::new(nr_level.raw() + delta);
        if new_val != nr_level {
            nr_level = new_val;
            NR_LEVEL.sender().send(nr_level);
        }
    }
}
