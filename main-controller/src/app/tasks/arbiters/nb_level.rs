use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::events::NB_LEVEL;
use crate::app::types::NbLevel;

pub static NB_LEVEL_CMD: Signal<ThreadModeRawMutex, i16> = Signal::new();

#[embassy_executor::task]
pub async fn nb_level_arbiter_task() {
    let mut nb_level = NbLevel::new(0);

    loop {
        let delta = NB_LEVEL_CMD.wait().await;
        let new_val = NbLevel::new(nb_level.raw() + delta);
        if new_val != nb_level {
            nb_level = new_val;
            NB_LEVEL.sender().send(nb_level);
        }
    }
}
