use embassy_stm32::{
    i2c::{mode as i2c_mode, I2c},
    mode::{self},
};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};

pub type MainBoardI2C = I2c<'static, mode::Async, i2c_mode::Master>;
pub type MainBoardI2CMutex = Mutex<ThreadModeRawMutex, MainBoardI2C>;

/// RSSI value in dBm (decibels relative to 1 milliwatt)
/// Typical range for HF receiver: -120 dBm (weak) to -20 dBm (very strong)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RssiDbm {
    pub dbm: i8, // dBm value (-128 to +127)
}

impl RssiDbm {
    pub fn from_adc_raw(raw: i16) -> Self {
        const ADC_MAX: i32 = 1656;
        const DBM_MIN: i32 = -120;
        const DBM_MAX: i32 = -20;
        const DBM_RANGE: i32 = DBM_MAX - DBM_MIN; // 100 dB

        let raw_clamped = raw.max(0) as i32;
        let dbm = DBM_MIN + (raw_clamped * DBM_RANGE / ADC_MAX);

        Self {
            dbm: dbm.clamp(-120, 0) as i8,
        }
    }
}
