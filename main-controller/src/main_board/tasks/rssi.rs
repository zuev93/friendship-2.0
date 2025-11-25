use crate::{
    app::events::CURRENT_MODE,
    main_board::{events::CURRENT_RSSI, modules::rssi::RssiReader},
};
use common::error::error;
use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use static_cell::StaticCell;

pub fn spawn_tasks(spawner: Spawner, rssi_reader: RssiReader) {
    static STATIC_CELL: StaticCell<Mutex<ThreadModeRawMutex, RssiReader>> = StaticCell::new();
    let mutex = STATIC_CELL.init(Mutex::new(rssi_reader));
    spawner.must_spawn(rssi_task_control(mutex));
    spawner.must_spawn(rssi_task_read(mutex));
}

#[embassy_executor::task]
async fn rssi_task_control(mutex: &'static Mutex<ThreadModeRawMutex, RssiReader>) {
    loop {
        let mode = CURRENT_MODE.wait().await;
        let mut reader = mutex.lock().await;
        if let Err(e) = reader.set_mode(mode).await {
            error(e).await;
        }
    }
}

#[embassy_executor::task]
async fn rssi_task_read(mutex: &'static Mutex<ThreadModeRawMutex, RssiReader>) {
    loop {
        let rssi = mutex.lock().await.read().await;
        if rssi.is_err() {
            error(rssi.err().unwrap()).await;
            continue;
        }
        let rssi_data = rssi.unwrap();
        let selected_rssi = select_rssi(rssi_data.rssi1, rssi_data.rssi2);
        CURRENT_RSSI.signal(selected_rssi);

        embassy_time::Timer::after_millis(10).await;
    }
}

fn select_rssi(
    rssi1: crate::main_board::types::RssiDbm,
    _rssi2: crate::main_board::types::RssiDbm,
) -> crate::main_board::types::RssiDbm {
    rssi1
}
