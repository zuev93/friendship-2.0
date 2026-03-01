use druzhba_macros::instrumented;
use crate::runtime_stats::TaskId;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::events::RF_POWER;
use crate::app::types::RfPowerPercent;

pub static RF_POWER_CMD: Signal<ThreadModeRawMutex, i16> = Signal::new();

#[instrumented(TaskId::RfPowerArbiter)]
#[embassy_executor::task]
pub async fn rf_power_arbiter_task() {
    let mut rf_power_raw: i16 = 0;

    loop {
        let delta = RF_POWER_CMD.wait().await;
        let new_raw = (rf_power_raw + delta).clamp(0, 1000);
        if new_raw != rf_power_raw {
            rf_power_raw = new_raw;
            RF_POWER.sender().send(RfPowerPercent::from_accumulated(rf_power_raw));
        }
    }
}
