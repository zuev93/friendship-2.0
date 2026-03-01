use core::sync::atomic::Ordering;

use embassy_stm32::wdg::IndependentWatchdog;
use embassy_time::Timer;

use crate::runtime_stats::{HEARTBEATS, MONITORED_TASKS, UPTIME_SECS};

#[embassy_executor::task]
pub async fn watchdog_task(mut wdg: IndependentWatchdog<'static, embassy_stm32::peripherals::IWDG>) {
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
