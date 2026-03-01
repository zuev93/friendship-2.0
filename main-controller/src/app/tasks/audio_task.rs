use embassy_executor::Spawner;
use embassy_futures::select::{select5, Either5};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use static_cell::StaticCell;

use crate::{
    app::{
        audio_mixer::AudioMixer,
        events::{
            AUDIO_BUFFER_HEADPHONES, AUDIO_BUFFER_SPEAKERS, AUDIO_BUFFER_TX, COMPRESSION,
            COMPRESSION_METER, SQUELCH, VOLUME,
        },
        tone_generator::ToneGenerator,
    },
    front_panel::events::{AUDIO_MIC_BUFFER, HEADPHONES_CONNECTED},
    main_board::events::{AUDIO_RX_BUFFER, CURRENT_RSSI},
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
        COMPRESSION_METER.signal(mixer.gain_reduction());

        AUDIO_BUFFER_TX.signal(mixer.get_buffer_tx());
        AUDIO_BUFFER_HEADPHONES.signal(mixer.get_buffer_headphones());
        AUDIO_BUFFER_SPEAKERS.signal(mixer.get_buffer_speakers());
    }
}

#[embassy_executor::task]
async fn controls_task(mutex: &'static Mutex<ThreadModeRawMutex, AudioMixer>) {
    loop {
        match select5(
            VOLUME.wait(),
            HEADPHONES_CONNECTED.wait(),
            SQUELCH.wait(),
            CURRENT_RSSI.wait(),
            COMPRESSION.wait(),
        )
        .await
        {
            Either5::First(volume) => {
                mutex.lock().await.set_volume(volume);
            }
            Either5::Second(connected) => {
                mutex.lock().await.set_headphones_connected(connected);
            }
            Either5::Third(squelch) => {
                let dbm = squelch_to_dbm(squelch.raw());
                mutex.lock().await.set_squelch_threshold(dbm);
            }
            Either5::Fourth(rssi) => {
                mutex.lock().await.update_squelch(rssi.dbm);
            }
            Either5::Fifth(compression) => {
                mutex.lock().await.set_compression(compression);
            }
        }
    }
}

fn squelch_to_dbm(raw: i16) -> i8 {
    const DBM_MIN: i32 = -120;
    const DBM_MAX: i32 = -20;
    const RAW_MAX: i32 = 1000;
    if raw <= 0 {
        return -128;
    }
    (DBM_MIN + (raw as i32 * (DBM_MAX - DBM_MIN) / RAW_MAX)) as i8
}
