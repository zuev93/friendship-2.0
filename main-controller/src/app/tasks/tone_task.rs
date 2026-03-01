use embassy_executor::Spawner;
use embassy_futures::select::{select5, Either5};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};

use crate::app::events::{BUTTON_BEEP, CW_PITCH, CW_SIDETONE_ACTIVE, MODE, TONE};
use crate::app::tone_generator::ToneGenerator;

pub fn spawn(spawner: Spawner, tone_generator: &'static Mutex<ThreadModeRawMutex, ToneGenerator>) {
    spawner.must_spawn(tone_control_task(tone_generator));
}

#[embassy_executor::task]
async fn tone_control_task(tone_generator: &'static Mutex<ThreadModeRawMutex, ToneGenerator>) {
    let mut mode_rcv = MODE.receiver().unwrap();
    let mut tone_rcv = TONE.receiver().unwrap();
    let mut beep_rcv = BUTTON_BEEP.receiver().unwrap();
    let mut sidetone_rcv = CW_SIDETONE_ACTIVE.receiver().unwrap();
    let mut pitch_rcv = CW_PITCH.receiver().unwrap();
    loop {
        match select5(
            mode_rcv.changed(),
            tone_rcv.changed(),
            beep_rcv.changed(),
            sidetone_rcv.changed(),
            pitch_rcv.changed(),
        )
        .await
        {
            Either5::First(mode) => tone_generator.lock().await.set_mode(mode),
            Either5::Second(active) => tone_generator.lock().await.set_tone_active(active),
            Either5::Third(_) => tone_generator.lock().await.trigger_beep(),
            Either5::Fourth(active) => tone_generator.lock().await.set_sidetone_active(active),
            Either5::Fifth(pitch) => tone_generator.lock().await.set_sidetone_freq(pitch.raw()),
        }
    }
}
