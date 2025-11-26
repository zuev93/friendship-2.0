use embassy_executor::Spawner;
use embassy_futures::select::{select3, Either3};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use embassy_time::Timer;
use static_cell::StaticCell;

use crate::app::events::{AUDIO_GENERATOR_OUT, BUTTON_BEEP, CURRENT_MODE, TONE_ACTIVE};
use crate::app::tone_generator::ToneGenerator;

pub fn spawn(spawner: Spawner) {
    static GEN: StaticCell<Mutex<ThreadModeRawMutex, ToneGenerator>> = StaticCell::new();
    let gen = GEN.init(Mutex::new(ToneGenerator::new()));
    spawner.must_spawn(tone_control_task(gen));
    spawner.must_spawn(tone_producer_task(gen));
}

#[embassy_executor::task]
async fn tone_control_task(gen: &'static Mutex<ThreadModeRawMutex, ToneGenerator>) {
    loop {
        match select3(CURRENT_MODE.wait(), TONE_ACTIVE.wait(), BUTTON_BEEP.wait()).await {
            Either3::First(mode) => {
                let mut g = gen.lock().await;
                g.set_mode(mode);
            }
            Either3::Second(active) => {
                let mut g: embassy_sync::mutex::MutexGuard<'_, ThreadModeRawMutex, ToneGenerator> =
                    gen.lock().await;
                g.set_tone_active(active);
            }
            Either3::Third(_) => {
                let mut g = gen.lock().await;
                g.trigger_beep();
            }
        }
    }
}

#[embassy_executor::task]
async fn tone_producer_task(gen: &'static Mutex<ThreadModeRawMutex, ToneGenerator>) {
    loop {
        Timer::after(ToneGenerator::buffer_period()).await;
        let mut g = gen.lock().await;
        let buffer = g.next_buffer();
        AUDIO_GENERATOR_OUT.signal(buffer);
    }
}
