use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::events::COMPRESSION;
use crate::app::types::Compression;

pub static COMPRESSION_CMD: Signal<ThreadModeRawMutex, i16> = Signal::new();

#[embassy_executor::task]
pub async fn compression_arbiter_task() {
    let mut compression = Compression::new(0);

    loop {
        let delta = COMPRESSION_CMD.wait().await;
        let new_val = Compression::new(compression.raw() + delta);
        if new_val != compression {
            compression = new_val;
            COMPRESSION.sender().send(compression);
        }
    }
}
