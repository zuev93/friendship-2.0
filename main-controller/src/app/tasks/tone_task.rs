use embassy_executor::Spawner;
use embassy_futures::select::{select3, Either3};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};

use crate::app::events::{BUTTON_BEEP, CURRENT_MODE, TONE_ACTIVE};
use crate::app::tone_generator::ToneGenerator;

pub fn spawn(spawner: Spawner, tone_generator: &'static Mutex<ThreadModeRawMutex, ToneGenerator>) {
    spawner.must_spawn(tone_control_task(tone_generator));
}

#[embassy_executor::task]
async fn tone_control_task(tone_generator: &'static Mutex<ThreadModeRawMutex, ToneGenerator>) {
    loop {
        match select3(CURRENT_MODE.wait(), TONE_ACTIVE.wait(), BUTTON_BEEP.wait()).await {
            Either3::First(mode) => tone_generator.lock().await.set_mode(mode),
            Either3::Second(active) => tone_generator.lock().await.set_tone_active(active),
            Either3::Third(_) => tone_generator.lock().await.trigger_beep(),
        }
    }
}
