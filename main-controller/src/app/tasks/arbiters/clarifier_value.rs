use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::events::CLARIFIER_VALUE;
use crate::app::types::ClarifierValue;

pub static CLARIFIER_VALUE_CMD: Signal<ThreadModeRawMutex, i16> = Signal::new();

#[embassy_executor::task]
pub async fn clarifier_value_arbiter_task() {
    let mut clarifier = ClarifierValue::new(0);

    loop {
        let delta = CLARIFIER_VALUE_CMD.wait().await;
        let new_val = ClarifierValue::new(clarifier.raw() + delta);
        if new_val != clarifier {
            clarifier = new_val;
            CLARIFIER_VALUE.signal(clarifier);
        }
    }
}
