use embassy_executor::Spawner;
use embassy_futures::select::{select, select4, select6, Either, Either4, Either6};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use static_cell::StaticCell;

use crate::{
    app::events::{FILTER, MODE, NB_ENABLED, NB_LEVEL, RF_GAIN_MODE, RF_POWER, NB_ACTIVE, TX_THERMAL_CONSTRAINT},
    control_board::events::{PD_CONTRACT, POWER_TELEMETRY},
    main_board::{events::CURRENT_RSSI, modules::crystall_filter::CrystallFilter},
};
use common::error::error;

pub fn spawn_tasks(spawner: Spawner, crystall_filter: CrystallFilter) {
    static CRYSTAL_FILTER: StaticCell<Mutex<ThreadModeRawMutex, CrystallFilter>> =
        StaticCell::new();
    let mutex = CRYSTAL_FILTER.init(Mutex::new(crystall_filter));
    spawner.must_spawn(crystall_filter_task(mutex));
    spawner.must_spawn(nb_activity_task(mutex));
}

#[embassy_executor::task]
async fn crystall_filter_task(mutex: &'static Mutex<ThreadModeRawMutex, CrystallFilter>) {
    loop {
        match select(
            select(
                select6(
                    RF_POWER.wait(),
                    POWER_TELEMETRY.wait(),
                    PD_CONTRACT.wait(),
                    TX_THERMAL_CONSTRAINT.wait(),
                    MODE.wait(),
                    FILTER.wait(),
                ),
                RF_GAIN_MODE.wait(),
            ),
            select4(
                NB_ENABLED.wait(),
                NB_LEVEL.wait(),
                CURRENT_RSSI.wait(),
                embassy_futures::yield_now(),
            ),
        )
        .await
        {
            Either::First(inner) => match inner {
                Either::First(inner6) => match inner6 {
                    Either6::First(rf_power) => {
                        if let Err(e) = mutex.lock().await.set_power(rf_power).await {
                            error(e).await;
                        }
                    }
                    Either6::Second(telemetry) => {
                        if let Err(e) = mutex.lock().await.set_power_telemetry(telemetry).await {
                            error(e).await;
                        }
                    }
                    Either6::Third(contract) => {
                        if let Err(e) = mutex.lock().await.set_pd_contract(contract).await {
                            error(e).await;
                        }
                    }
                    Either6::Fourth(thermal) => {
                        if let Err(e) = mutex.lock().await.set_thermal_constraint(thermal).await {
                            error(e).await;
                        }
                    }
                    Either6::Fifth(mode) => {
                        if let Err(e) = mutex.lock().await.set_mode(mode).await {
                            error(e).await;
                        }
                    }
                    Either6::Sixth(filter) => {
                        if let Err(e) = mutex.lock().await.set_filter_type(filter).await {
                            error(e).await;
                        }
                    }
                },
                Either::Second(rf_gain_mode) => {
                    if let Err(e) = mutex.lock().await.set_rf_gain_mode(rf_gain_mode).await {
                        error(e).await;
                    }
                }
            },
            Either::Second(nb_inner) => match nb_inner {
                Either4::First(enabled) => {
                    if let Err(e) = mutex.lock().await.set_nb_enabled(enabled).await {
                        error(e).await;
                    }
                }
                Either4::Second(nb_level) => {
                    if let Err(e) = mutex.lock().await.set_nb_level(nb_level).await {
                        error(e).await;
                    }
                }
                Either4::Third(rssi) => {
                    if let Err(e) = mutex.lock().await.set_rssi(rssi).await {
                        error(e).await;
                    }
                }
                Either4::Fourth(()) => {}
            },
        }
    }
}

#[embassy_executor::task]
async fn nb_activity_task(mutex: &'static Mutex<ThreadModeRawMutex, CrystallFilter>) {
    loop {
        match mutex.lock().await.read_nb_activity().await {
            Ok(active) => {
                NB_ACTIVE.signal(active);
            }
            Err(e) => {
                error(e).await;
            }
        }
        embassy_time::Timer::after_millis(100).await;
    }
}
