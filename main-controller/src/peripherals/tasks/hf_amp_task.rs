use embassy_executor::Spawner;
use embassy_futures::select::{select, select4, Either, Either4};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use embassy_time::{Duration, Timer};
use static_cell::StaticCell;

use crate::{
    app::events::{CURRENT_MODE, CURRENT_RF_POWER, PA_TEMPERATURES},
    control_board::events::{
        EmergencyReason, PdContract, EMERGENCY_SHUTDOWN, PD_CONTRACT, POWER_TELEMETRY,
    },
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
    let mut last_telemetry = crate::control_board::events::PowerTelemetry::default();
    let mut last_contract = PdContract::default();

    loop {
        let normal = select4(
            CURRENT_MODE.wait(),
            CURRENT_RF_POWER.wait(),
            POWER_TELEMETRY.wait(),
            PD_CONTRACT.wait(),
        );

        match select(normal, EMERGENCY_SHUTDOWN.wait()).await {
            Either::First(normal_result) => match normal_result {
                Either4::First(mode) => {
                    if let Err(e) = mutex.lock().await.set_mode(mode).await {
                        error(e).await;
                    }
                }
                Either4::Second(power) => {
                    if let Err(e) = mutex.lock().await.set_user_power(power).await {
                        error(e).await;
                    }
                }
                Either4::Third(telemetry) => {
                    last_telemetry = telemetry;
                    let limit = HfAmp::derive_power_budget(&last_telemetry, &last_contract);
                    if let Err(e) = mutex.lock().await.set_power_budget(limit).await {
                        error(e).await;
                    }
                }
                Either4::Fourth(contract) => {
                    last_contract = contract;
                    let limit = HfAmp::derive_power_budget(&last_telemetry, &last_contract);
                    if let Err(e) = mutex.lock().await.set_power_budget(limit).await {
                        error(e).await;
                    }
                }
            },
            Either::Second(reason) => {
                mutex.lock().await.emergency_off().await;
                match reason {
                    EmergencyReason::VbusOvercurrent => {
                        error("HfAmp: emergency off - VBUS overcurrent").await;
                    }
                    EmergencyReason::PaOvercurrent => {
                        error("HfAmp: emergency off - PA overcurrent").await;
                    }
                    EmergencyReason::Thermal => {
                        error("HfAmp: emergency off - thermal").await;
                    }
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
            Ok(temps) => {
                if HfAmp::is_thermal_emergency(&temps) {
                    EMERGENCY_SHUTDOWN.signal(EmergencyReason::Thermal);
                }
                PA_TEMPERATURES.signal(temps);
            }
            Err(e) => error(e).await,
        }
    }
}
