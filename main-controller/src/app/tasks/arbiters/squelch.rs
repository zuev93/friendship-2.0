use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::events::SQUELCH;
use crate::app::types::Squelch;

pub static SQUELCH_CMD: Signal<ThreadModeRawMutex, i16> = Signal::new();

#[embassy_executor::task]
pub async fn squelch_arbiter_task() {
    let mut squelch = Squelch::new(0);

    loop {
        let delta = SQUELCH_CMD.wait().await;
        let new_val = Squelch::new(squelch.raw() + delta);
        if new_val != squelch {
            squelch = new_val;
            SQUELCH.signal(squelch);
        }
    }
}
