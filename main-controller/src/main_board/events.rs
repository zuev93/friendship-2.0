use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::watch::Watch;

use crate::consts::ADC_BUFFER_SIZE;
use crate::main_board::types::RssiDbm;

pub static CURRENT_RSSI1: Watch<ThreadModeRawMutex, RssiDbm, 2> = Watch::new();
pub static CURRENT_RSSI2: Watch<ThreadModeRawMutex, RssiDbm, 6> = Watch::new();
pub static AUDIO_RX_BUFFER: Watch<ThreadModeRawMutex, [u32; ADC_BUFFER_SIZE], 2> = Watch::new();
pub static AGC_DAC_VALUE: Watch<ThreadModeRawMutex, u16, 2> = Watch::new();
