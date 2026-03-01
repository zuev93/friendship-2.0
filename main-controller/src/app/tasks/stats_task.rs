use druzhba_macros::instrumented;
use crate::runtime_stats::TaskId;
use core::sync::atomic::Ordering;
use embassy_time::Timer;

use crate::app::events::{CPU_PERCENT, RAM_STATS};
use crate::app::tasks::idle_counter_task::IDLE_COUNTER;
use crate::app::types::CpuPercent;
use crate::runtime_stats::{self, IDLE_CAL};

#[instrumented(TaskId::StatsTask)]
#[embassy_executor::task]
pub async fn stats_task() {
    loop {
        Timer::after_secs(1).await;
        let idle = IDLE_COUNTER.swap(0, Ordering::Relaxed);
        let cal = IDLE_CAL.load(Ordering::Relaxed);
        let cpu_pct = if cal > 0 {
            100u8.saturating_sub((idle.min(cal) * 100 / cal) as u8)
        } else {
            0
        };
        CPU_PERCENT.sender().send(CpuPercent::new(cpu_pct));
        RAM_STATS.sender().send(runtime_stats::ram_stats());
    }
}
