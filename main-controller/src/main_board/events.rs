use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::watch::Watch;

use crate::consts::AUDIO_BUFFER_SIZE;
use crate::main_board::types::RssiDbm;

pub static CURRENT_RSSI1: Watch<ThreadModeRawMutex, RssiDbm, 2> = Watch::new();
pub static CURRENT_RSSI2: Watch<ThreadModeRawMutex, RssiDbm, 6> = Watch::new();
pub static AUDIO_RX_BUFFER: Watch<ThreadModeRawMutex, [u16; AUDIO_BUFFER_SIZE], 2> = Watch::new();
