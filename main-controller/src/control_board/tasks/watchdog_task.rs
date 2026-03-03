use core::sync::atomic::Ordering;

use embassy_executor::Spawner;
use embassy_time::Timer;

use crate::control_board::modules::watchdog::Watchdog;
use crate::runtime_stats::{HEARTBEATS, MONITORED_TASKS, UPTIME_SECS};

pub fn create_task(spawner: Spawner, watchdog: Watchdog) {
    spawner.must_spawn(watchdog_task(watchdog));
}

#[embassy_executor::task]
async fn watchdog_task(mut wdg: Watchdog) {
    wdg.unleash();
    let mut snapshot = [0u32; MONITORED_TASKS.len()];

    for (i, &task) in MONITORED_TASKS.iter().enumerate() {
        snapshot[i] = HEARTBEATS[task as usize].load(Ordering::Relaxed);
    }

    loop {
        Timer::after_secs(1).await;

        let mut all_alive = true;
        for (i, &task) in MONITORED_TASKS.iter().enumerate() {
            let current = HEARTBEATS[task as usize].load(Ordering::Relaxed);
            if current == snapshot[i] {
                all_alive = false;
                break;
            }
            snapshot[i] = current;
        }

        if all_alive {
            wdg.pet();
        } else {
            let uptime = UPTIME_SECS.load(Ordering::Relaxed);
            crate::crash_info::write_watchdog(uptime);
        }
    }
}
