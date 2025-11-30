use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use embassy_time::Timer;
use static_cell::StaticCell;

use crate::{
    app::events::CURRENT_MODE, control_board::events::POWER_TELEMETRY,
    control_board::modules::power_control::PowerControl,
};
use common::error::error;

const TELEMETRY_PERIOD_MS: u64 = 250;

pub fn create_tasks(spawner: Spawner, power_control: PowerControl) {
    static STATIC_CELL: StaticCell<Mutex<ThreadModeRawMutex, PowerControl>> = StaticCell::new();
    let mutex = STATIC_CELL.init(Mutex::new(power_control));

    spawner.must_spawn(power_control_task(mutex));
    spawner.must_spawn(power_telemetry_task(mutex));
}

#[embassy_executor::task]
async fn power_control_task(mutex: &'static Mutex<ThreadModeRawMutex, PowerControl>) {
    loop {
        let mode = CURRENT_MODE.wait().await;
        if let Err(_) = mutex.lock().await.set_mode(mode).await {
            error("Failed to set mode to power control module").await;
        }
    }
}

#[embassy_executor::task]
async fn power_telemetry_task(mutex: &'static Mutex<ThreadModeRawMutex, PowerControl>) {
    loop {
        let telemetry = mutex.lock().await.read_power_telemetry().await;
        match telemetry {
            Ok(data) => POWER_TELEMETRY.signal(data),
            Err(e) => error(e).await,
        }

        Timer::after_millis(TELEMETRY_PERIOD_MS).await;
    }
}
