use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::consts::AUDIO_BUFFER_SIZE;
use crate::main_board::types::RssiDbm;

pub static CURRENT_RSSI: Signal<ThreadModeRawMutex, RssiDbm> = Signal::new();
pub static AUDIO_RX_BUFFER: Signal<ThreadModeRawMutex, [u16; AUDIO_BUFFER_SIZE]> = Signal::new();
