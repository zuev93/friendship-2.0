use embassy_executor::Spawner;
use embassy_futures::select::{select4, Either4};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use static_cell::StaticCell;

use crate::{
    app::events::{IF_GAIN, IF_GAIN_MODE, MODE, RSSI_FAST_MODE},
    main_board::{events::CURRENT_RSSI, modules::if_amplifier::IfAmplifier},
};
use common::error::error;

pub fn spawn_tasks(spawner: Spawner, if_gain_control: IfAmplifier) {
    static IF_AMPLIFIER: StaticCell<Mutex<ThreadModeRawMutex, IfAmplifier>> = StaticCell::new();
    let mutex = IF_AMPLIFIER.init(Mutex::new(if_gain_control));
    spawner.must_spawn(if_gain_control_task(mutex));
    spawner.must_spawn(rssi_task_read(mutex));
}

#[embassy_executor::task]
async fn if_gain_control_task(mutex: &'static Mutex<ThreadModeRawMutex, IfAmplifier>) {
    loop {
        match select4(
            IF_GAIN.wait(),
            IF_GAIN_MODE.wait(),
            CURRENT_RSSI.wait(),
            MODE.wait(),
        )
        .await
        {
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
        }
    }
}

#[embassy_executor::task]
async fn rssi_task_read(mutex: &'static Mutex<ThreadModeRawMutex, IfAmplifier>) {
    loop {
        let rssi_data = match mutex.lock().await.read_rssi().await {
            Ok(data) => data,
            Err(e) => {
                error(e).await;
                continue;
            }
        };
        let selected_rssi = select_rssi(rssi_data.rssi1, rssi_data.rssi2);
        CURRENT_RSSI.signal(selected_rssi);

        if RSSI_FAST_MODE.signaled() {
            embassy_futures::yield_now().await;
        } else {
            embassy_time::Timer::after_millis(10).await;
        }
    }
}

fn select_rssi(
    rssi1: crate::main_board::types::RssiDbm,
    _rssi2: crate::main_board::types::RssiDbm,
) -> crate::main_board::types::RssiDbm {
    rssi1
}
