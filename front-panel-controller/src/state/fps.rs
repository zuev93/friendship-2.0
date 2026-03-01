use core::sync::atomic::{AtomicU32, Ordering};

pub static FPS_COUNTER: [AtomicU32; 3] = [AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0)];

pub fn increment(display_index: usize) {
    FPS_COUNTER[display_index].fetch_add(1, Ordering::Relaxed);
}
