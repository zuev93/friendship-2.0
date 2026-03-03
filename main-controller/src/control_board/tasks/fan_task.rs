use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use static_cell::StaticCell;

use crate::{
    app::events::PA_TEMPERATURES,
    control_board::modules::fan::Fan,
    runtime_stats::TaskId,
};
use druzhba_macros::instrumented;

pub fn create_task(spawner: Spawner, fan: Fan) {
    static FAN: StaticCell<Mutex<ThreadModeRawMutex, Fan>> = StaticCell::new();
    let mutex = FAN.init(Mutex::new(fan));
    spawner.must_spawn(fan_control_task(mutex));
}

#[instrumented(TaskId::FanControl)]
#[embassy_executor::task]
async fn fan_control_task(mutex: &'static Mutex<ThreadModeRawMutex, Fan>) {
    let mut temp_rcv = PA_TEMPERATURES.receiver().unwrap();
    loop {
        let temps = temp_rcv.changed().await;
        mutex.lock().await.update(temps.driver_c, temps.final_c);
    }
}
