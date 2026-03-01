use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use embassy_time::{Duration, Timer};
use static_cell::StaticCell;

use crate::{
    app::events::{COUPLER_METRICS, FREQUENCY, MODE},
    peripherals::modules::lpf::Lpf,
};
use common::error::error;

const COUPLER_SAMPLE_PERIOD: Duration = Duration::from_millis(200);

pub fn create_tasks(spawner: Spawner, lpf: Lpf) {
    static LPF: StaticCell<Mutex<ThreadModeRawMutex, Lpf>> = StaticCell::new();
    let mutex = LPF.init(Mutex::new(lpf));

    spawner.must_spawn(lpf_control_task(mutex));
    spawner.must_spawn(lpf_data_task(mutex));
}

#[embassy_executor::task]
async fn lpf_control_task(mutex: &'static Mutex<ThreadModeRawMutex, Lpf>) {
    let mut frequency_rcv = FREQUENCY.receiver().unwrap();
    let mut mode_rcv = MODE.receiver().unwrap();
    loop {
        match select(frequency_rcv.changed(), mode_rcv.changed()).await {
            Either::First(frequency) => {
                if let Err(e) = mutex.lock().await.set_frequency(frequency).await {
                    error(e).await;
                }
            }
            Either::Second(mode) => {
                if let Err(e) = mutex.lock().await.set_mode(mode).await {
                    error(e).await;
                }
            }
        }
    }
}

#[embassy_executor::task]
async fn lpf_data_task(mutex: &'static Mutex<ThreadModeRawMutex, Lpf>) {
    loop {
        Timer::after(COUPLER_SAMPLE_PERIOD).await;
        match mutex.lock().await.read_coupler_metrics().await {
            Ok(metrics) => COUPLER_METRICS.sender().send(metrics),
            Err(e) => error(e).await,
        }
    }
}
