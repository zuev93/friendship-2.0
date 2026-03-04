use embassy_executor::Spawner;
use embassy_futures::select::{select3, Either3};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use static_cell::StaticCell;

use crate::{
    app::events::{MODE, NB_LEVEL, NB_ACTIVE},
    main_board::{events::CURRENT_RSSI1, modules::crystal_filter::CrystalFilter},
    runtime_stats::TaskId,
};
use common::error::error;
use druzhba_macros::instrumented;

pub fn spawn_tasks(spawner: Spawner, crystal_filter: CrystalFilter) {
    static CRYSTAL_FILTER: StaticCell<Mutex<ThreadModeRawMutex, CrystalFilter>> =
        StaticCell::new();
    let mutex = CRYSTAL_FILTER.init(Mutex::new(crystal_filter));
    spawner.must_spawn(crystal_filter_task(mutex));
    spawner.must_spawn(nb_activity_task(mutex));
}

#[instrumented(TaskId::CrystalFilter)]
#[embassy_executor::task]
async fn crystal_filter_task(mutex: &'static Mutex<ThreadModeRawMutex, CrystalFilter>) {
    let mut mode_rcv = MODE.receiver().unwrap();
    let mut nb_level_rcv = NB_LEVEL.receiver().unwrap();
    let mut rssi_rcv = CURRENT_RSSI1.receiver().unwrap();
    loop {
        match select3(
            mode_rcv.changed(),
            nb_level_rcv.changed(),
            rssi_rcv.changed(),
        )
        .await
        {
            Either3::First(mode) => {
                if let Err(e) = mutex.lock().await.set_mode(mode).await {
                    error(e).await;
                }
            }
            Either3::Second(nb_level) => {
                if let Err(e) = mutex.lock().await.set_nb_level(nb_level).await {
                    error(e).await;
                }
            }
            Either3::Third(rssi) => {
                if let Err(e) = mutex.lock().await.set_rssi(rssi).await {
                    error(e).await;
                }
            }
        }
    }
}

#[instrumented(TaskId::NbActivity)]
#[embassy_executor::task]
async fn nb_activity_task(mutex: &'static Mutex<ThreadModeRawMutex, CrystalFilter>) {
    loop {
        match mutex.lock().await.read_nb_activity().await {
            Ok(active) => {
                NB_ACTIVE.sender().send(active);
            }
            Err(e) => {
                error(e).await;
            }
        }
        embassy_time::Timer::after_millis(100).await;
    }
}
