use core::sync::atomic::{AtomicU32, Ordering};
use druzhba_common::PlatformMutex;
use embassy_sync::mutex::Mutex;

const LOG_CAPACITY: usize = 16;

struct ErrorLogInner {
    entries: [&'static str; LOG_CAPACITY],
    write_pos: usize,
    stored: usize,
}

pub struct ErrorLog {
    total: AtomicU32,
    inner: Mutex<PlatformMutex, ErrorLogInner>,
}

impl ErrorLog {
    pub const fn new() -> Self {
        Self {
            total: AtomicU32::new(0),
            inner: Mutex::new(ErrorLogInner {
                entries: [""; LOG_CAPACITY],
                write_pos: 0,
                stored: 0,
            }),
        }
    }

    pub async fn push(&self, message: &'static str) {
        self.total.fetch_add(1, Ordering::Relaxed);
        let mut inner = self.inner.lock().await;
        let pos = inner.write_pos;
        inner.entries[pos] = message;
        inner.write_pos = (inner.write_pos + 1) % LOG_CAPACITY;
        if inner.stored < LOG_CAPACITY {
            inner.stored += 1;
        }
    }

    pub fn total(&self) -> u32 {
        self.total.load(Ordering::Relaxed)
    }

    pub async fn recent(&self, out: &mut [&'static str]) -> usize {
        let inner = self.inner.lock().await;
        let count = inner.stored.min(out.len());
        let start = if inner.stored >= count {
            (inner.write_pos + LOG_CAPACITY - count) % LOG_CAPACITY
        } else {
            0
        };
        for i in 0..count {
            out[i] = inner.entries[(start + i) % LOG_CAPACITY];
        }
        count
    }
}
