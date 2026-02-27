use embassy_executor::Spawner;
use embassy_futures::select::{select3, Either3};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use embassy_time::{Duration, Timer};
use static_cell::StaticCell;

use crate::{
    app::events::{CURRENT_MODE, CURRENT_RF_POWER, PA_TEMPERATURES},
    control_board::events::POWER_TELEMETRY,
    peripherals::modules::hf_amp::HfAmp,
};
use common::error::error;

const THERMAL_SAMPLE_PERIOD: Duration = Duration::from_millis(500);

pub fn create_tasks(spawner: Spawner, hf_amp: HfAmp) {
    static STATIC_CELL: StaticCell<Mutex<ThreadModeRawMutex, HfAmp>> = StaticCell::new();
    let mutex = STATIC_CELL.init(Mutex::new(hf_amp));

    spawner.must_spawn(hf_amp_control_task(mutex));
    spawner.must_spawn(hf_amp_thermal_task(mutex));
}

#[embassy_executor::task]
async fn hf_amp_control_task(mutex: &'static Mutex<ThreadModeRawMutex, HfAmp>) {
    loop {
        match select3(
            CURRENT_MODE.wait(),
            CURRENT_RF_POWER.wait(),
            POWER_TELEMETRY.wait(),
        )
        .await
        {
            Either3::First(mode) => {
                if let Err(e) = mutex.lock().await.set_mode(mode).await {
                    error(e).await;
                }
            }
            Either3::Second(power) => {
                if let Err(e) = mutex.lock().await.set_user_power(power).await {
                    error(e).await;
                }
            }
            Either3::Third(telemetry) => {
                let limit = HfAmp::derive_power_budget(&telemetry);
                if let Err(e) = mutex.lock().await.set_power_budget(limit).await {
                    error(e).await;
                }
            }
        }
    }
}

#[embassy_executor::task]
async fn hf_amp_thermal_task(mutex: &'static Mutex<ThreadModeRawMutex, HfAmp>) {
    loop {
        Timer::after(THERMAL_SAMPLE_PERIOD).await;
        match mutex.lock().await.read_and_update_temperatures().await {
            Ok(temps) => PA_TEMPERATURES.signal(temps),
            Err(e) => error(e).await,
        }
    }
}
