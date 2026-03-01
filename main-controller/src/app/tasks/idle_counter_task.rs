use druzhba_macros::instrumented;
use crate::runtime_stats::TaskId;
use core::sync::atomic::{AtomicU32, Ordering};

pub static IDLE_COUNTER: AtomicU32 = AtomicU32::new(0);

#[instrumented(TaskId::IdleCounter)]
#[embassy_executor::task]
pub async fn idle_counter_task() {
    loop {
        embassy_futures::yield_now().await;
        IDLE_COUNTER.fetch_add(1, Ordering::Relaxed);
    }
}
