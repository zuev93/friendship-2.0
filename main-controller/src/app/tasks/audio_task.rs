use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use static_cell::StaticCell;

use crate::{
    app::{
        audio_mixer::AudioMixer,
        events::{AUDIO_BUFFER_HEADPHONES, AUDIO_BUFFER_SPEAKERS, AUDIO_BUFFER_TX, CURRENT_VOLUME},
        tone_generator::ToneGenerator,
    },
    front_panel::events::{AUDIO_MIC_BUFFER, HEADPHONES_CONNECTED},
    main_board::events::AUDIO_RX_BUFFER,
};

pub fn spawn_tasks(
    spawner: Spawner,
    tone_generator: &'static Mutex<ThreadModeRawMutex, ToneGenerator>,
) {
    static MIXER: StaticCell<Mutex<ThreadModeRawMutex, AudioMixer>> = StaticCell::new();
    let mixer = MIXER.init(Mutex::new(AudioMixer::new()));
    spawner.must_spawn(audio_task(mixer, tone_generator));
    spawner.must_spawn(controls_task(mixer));
}

#[embassy_executor::task]
async fn audio_task(
    mutex: &'static Mutex<ThreadModeRawMutex, AudioMixer>,
    tone_generator: &'static Mutex<ThreadModeRawMutex, ToneGenerator>,
) {
    loop {
        let audio_rx = AUDIO_RX_BUFFER.wait().await;
        let mic = AUDIO_MIC_BUFFER.wait().await;
        let generator = tone_generator.lock().await.next_buffer();

        let mut mixer = mutex.lock().await;
        mixer.set_buffer_rx(audio_rx);
        mixer.set_buffer_generator(generator);
        mixer.set_buffer_mic(mic);

        mixer.mix();

        AUDIO_BUFFER_TX.signal(mixer.get_buffer_tx());
        AUDIO_BUFFER_HEADPHONES.signal(mixer.get_buffer_headphones());
        AUDIO_BUFFER_SPEAKERS.signal(mixer.get_buffer_speakers());
    }
}

#[embassy_executor::task]
async fn controls_task(mutex: &'static Mutex<ThreadModeRawMutex, AudioMixer>) {
    loop {
        match select(CURRENT_VOLUME.wait(), HEADPHONES_CONNECTED.wait()).await {
            Either::First(volume) => {
                mutex.lock().await.set_volume(volume);
            }
            Either::Second(connected) => {
                mutex.lock().await.set_headphones_connected(connected);
            }
        }
    }
}
