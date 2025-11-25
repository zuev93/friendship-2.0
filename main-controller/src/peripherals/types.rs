#![allow(dead_code)]

use embassy_stm32::{
    i2c::{mode as i2c_mode, I2c},
    mode,
};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};

/// LPF (Low Pass Filter) cutoff frequencies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LpfCutoff {
    Cutoff1_8MHz,
    Cutoff3_5MHz,
    Cutoff7MHz,
    Cutoff14MHz,
    Cutoff21MHz,
    Cutoff28MHz,
    Bypass,
}

impl LpfCutoff {
    /// Get LPF cutoff from frequency in Hz
    pub fn from_frequency(freq_hz: u32) -> Self {
        match freq_hz {
            0..=2_000_000 => Self::Cutoff1_8MHz,
            2_000_001..=5_000_000 => Self::Cutoff3_5MHz,
            5_000_001..=10_000_000 => Self::Cutoff7MHz,
            10_000_001..=18_000_000 => Self::Cutoff14MHz,
            18_000_001..=24_000_000 => Self::Cutoff21MHz,
            24_000_001..=30_000_000 => Self::Cutoff28MHz,
            _ => Self::Bypass,
        }
    }
}

pub type PeripherialI2c =
    &'static Mutex<ThreadModeRawMutex, I2c<'static, mode::Async, i2c_mode::Master>>;
