use embassy_executor::Spawner;
use embassy_futures::select::{select, select3, select4, Either, Either3, Either4};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use static_cell::StaticCell;

use crate::{
    app::events::{ALC_CONSTRAINT, IF_GAIN, IF_GAIN_MODE, MODE, RF_POWER, RSSI_FAST_MODE, TX_THERMAL_CONSTRAINT},
    control_board::events::{PD_CONTRACT, POWER_TELEMETRY},
    main_board::{
        events::{CURRENT_RSSI1, CURRENT_RSSI2},
        modules::if_amplifier::IfAmplifier,
    },
    runtime_stats::TaskId,
};
use common::error::error;
use druzhba_macros::instrumented;

pub fn spawn_tasks(spawner: Spawner, if_amplifier: IfAmplifier) {
    static IF_AMPLIFIER: StaticCell<Mutex<ThreadModeRawMutex, IfAmplifier>> = StaticCell::new();
    let mutex = IF_AMPLIFIER.init(Mutex::new(if_amplifier));
    spawner.must_spawn(if_amplifier_task(mutex));
    spawner.must_spawn(rssi_task_read(mutex));
}

#[instrumented(TaskId::IfGainControl)]
#[embassy_executor::task]
async fn if_amplifier_task(mutex: &'static Mutex<ThreadModeRawMutex, IfAmplifier>) {
    let mut if_gain_rcv = IF_GAIN.receiver().unwrap();
    let mut if_gain_mode_rcv = IF_GAIN_MODE.receiver().unwrap();
    let mut rssi_rcv = CURRENT_RSSI2.receiver().unwrap();
    let mut mode_rcv = MODE.receiver().unwrap();
    let mut rf_power_rcv = RF_POWER.receiver().unwrap();
    let mut power_telemetry_rcv = POWER_TELEMETRY.receiver().unwrap();
    let mut pd_contract_rcv = PD_CONTRACT.receiver().unwrap();
    let mut tx_thermal_rcv = TX_THERMAL_CONSTRAINT.receiver().unwrap();
    let mut alc_rcv = ALC_CONSTRAINT.receiver().unwrap();
    loop {
        match select(
            select4(
                if_gain_rcv.changed(),
                if_gain_mode_rcv.changed(),
                rssi_rcv.changed(),
                mode_rcv.changed(),
            ),
            select(
                select3(
                    rf_power_rcv.changed(),
                    power_telemetry_rcv.changed(),
                    pd_contract_rcv.changed(),
                ),
                select(tx_thermal_rcv.changed(), alc_rcv.changed()),
            ),
        )
        .await
        {
            Either::First(inner) => match inner {
                Either4::First(if_gain) => {
                    if let Err(e) = mutex.lock().await.set_manual_gain_raw(if_gain).await {
                        error(e).await;
                    }
                }
                Either4::Second(if_gain_mode) => {
                    if let Err(e) = mutex.lock().await.set_if_gain_mode(if_gain_mode).await {
                        error(e).await;
                    }
                }
                Either4::Third(rssi) => {
                    if let Err(e) = mutex.lock().await.update_agc(rssi).await {
                        error(e).await;
                    }
                }
                Either4::Fourth(mode) => {
                    if let Err(e) = mutex.lock().await.set_mode(mode).await {
                        error(e).await;
                    }
                }
            },
            Either::Second(inner) => match inner {
                Either::First(inner2) => match inner2 {
                    Either3::First(rf_power) => {
                        if let Err(e) = mutex.lock().await.set_power(rf_power).await {
                            error(e).await;
                        }
                    }
                    Either3::Second(telemetry) => {
                        if let Err(e) = mutex.lock().await.set_power_telemetry(telemetry).await {
                            error(e).await;
                        }
                    }
                    Either3::Third(contract) => {
                        if let Err(e) = mutex.lock().await.set_pd_contract(contract).await {
                            error(e).await;
                        }
                    }
                },
                Either::Second(inner2) => match inner2 {
                    Either::First(thermal) => {
                        if let Err(e) = mutex.lock().await.set_thermal_constraint(thermal).await {
                            error(e).await;
                        }
                    }
                    Either::Second(alc) => {
                        if let Err(e) = mutex.lock().await.set_alc_constraint(alc).await {
                            error(e).await;
                        }
                    }
                },
            },
        }
    }
}

#[instrumented(TaskId::RssiRead)]
#[embassy_executor::task]
async fn rssi_task_read(mutex: &'static Mutex<ThreadModeRawMutex, IfAmplifier>) {
    let mut rssi_fast_rcv = RSSI_FAST_MODE.receiver().unwrap();
    loop {
        let rssi_data = match mutex.lock().await.read_rssi().await {
            Ok(data) => data,
            Err(e) => {
                error(e).await;
                continue;
            }
        };
        CURRENT_RSSI1.sender().send(rssi_data.rssi1);
        CURRENT_RSSI2.sender().send(rssi_data.rssi2);

        if rssi_fast_rcv.try_changed().is_some() {
            embassy_futures::yield_now().await;
        } else {
            embassy_time::Timer::after_millis(10).await;
        }
    }
}
