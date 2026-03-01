use core::sync::atomic::Ordering;
use embassy_executor::Spawner;
use embassy_time::Timer;

use crate::state::fps::FPS_COUNTER;
use crate::state::output::{DisplayFpsEvent, OutputEvent, OUTPUT_EVENTS};

pub fn spawn_tasks(spawner: &Spawner) {
    spawner.must_spawn(fps_task());
}

#[embassy_executor::task]
async fn fps_task() {
    loop {
        Timer::after_secs(1).await;
        let fps = [
            FPS_COUNTER[0].swap(0, Ordering::Relaxed) as u16,
            FPS_COUNTER[1].swap(0, Ordering::Relaxed) as u16,
            FPS_COUNTER[2].swap(0, Ordering::Relaxed) as u16,
        ];
        OUTPUT_EVENTS.send(OutputEvent::DisplayFps(DisplayFpsEvent { fps })).await;
    }
}
