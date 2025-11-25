use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use static_cell::StaticCell;

use crate::{
    app::{
        audio_mixer::AudioMixer,
        events::{
            AUDIO_BUFFER_HEADPHONES, AUDIO_BUFFER_SPEAKERS, AUDIO_BUFFER_TX, AUDIO_GENERATOR_OUT,
        },
    },
    main_board::events::{AUDIO_MIC_BUFFER, AUDIO_RX_BUFFER},
};

pub fn spawn_tasks(spawner: Spawner) {
    static MIXER: StaticCell<Mutex<ThreadModeRawMutex, AudioMixer>> = StaticCell::new();
    let mixer = MIXER.init(Mutex::new(AudioMixer::new()));
    spawner.must_spawn(audio_task(mixer));
    spawner.must_spawn(audio_modes_task(mixer));
}

// TODO check whether ThreadModeRawMutex is a right choce
#[embassy_executor::task]
async fn audio_task(mutex: &'static Mutex<ThreadModeRawMutex, AudioMixer>) {
    loop {
        let audio_rx = AUDIO_RX_BUFFER.wait().await;
        let generator = AUDIO_GENERATOR_OUT.wait().await;
        let mic = AUDIO_MIC_BUFFER.wait().await;

        let mixer = mutex.lock().await;
        mixer.set_buffer_rx(audio_rx);
        mixer.set_buffer_generator(generator);
        mixer.set_buffer_mic(mic);

        AUDIO_BUFFER_TX.signal(mixer.get_buffer_tx());
        AUDIO_BUFFER_HEADPHONES.signal(mixer.get_buffer_headphones());
        AUDIO_BUFFER_SPEAKERS.signal(mixer.get_buffer_speakers());
    }
}

#[embassy_executor::task]
async fn audio_modes_task(mutex: &'static Mutex<ThreadModeRawMutex, AudioMixer>) {
    loop {
        let _mixer = mutex.lock().await;
        // mixer.setMode(mode);
    }
}
