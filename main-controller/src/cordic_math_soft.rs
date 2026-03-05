use core::cell::RefCell;
use embassy_sync::blocking_mutex::{raw::CriticalSectionRawMutex, Mutex};

const LN10: f32 = 2.302_585;
const SQRT_2: f32 = 1.414_213_5;

pub type CordicMutex = Mutex<CriticalSectionRawMutex, RefCell<CordicMath>>;

pub fn with_cordic<R>(mutex: &'static CordicMutex, f: impl FnOnce(&mut CordicMath) -> R) -> R {
    mutex.lock(|cell| f(&mut cell.borrow_mut()))
}

pub struct CordicMath;

impl CordicMath {
    pub fn new() -> Self {
        Self
    }

    pub fn atan2f(&mut self, y: f32, x: f32) -> f32 {
        libm::atan2f(y, x)
    }

    pub fn sinf(&mut self, radians: f32) -> f32 {
        libm::sinf(radians)
    }

    pub fn cosf(&mut self, radians: f32) -> f32 {
        libm::cosf(radians)
    }

    pub fn sin_cos(&mut self, radians: f32) -> (f32, f32) {
        (libm::sinf(radians), libm::cosf(radians))
    }

    pub fn sqrtf(&mut self, x: f32) -> f32 {
        if x <= 0.0 {
            return 0.0;
        }
        libm::sqrtf(x)
    }

    pub fn lnf(&mut self, x: f32) -> f32 {
        if x <= 0.0 {
            return f32::NEG_INFINITY;
        }
        libm::logf(x)
    }

    pub fn expf(&mut self, x: f32) -> f32 {
        libm::expf(x)
    }

    pub fn pow10f(&mut self, x: f32) -> f32 {
        self.expf(x * LN10)
    }

    pub fn db_to_amplitude(&mut self, db: f32) -> f32 {
        self.pow10f(db / 20.0)
    }

    pub fn sqrt_2() -> f32 {
        SQRT_2
    }
}
