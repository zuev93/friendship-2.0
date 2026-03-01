use embassy_executor::Spawner;
use embassy_futures::select::{select, select5, Either, Either5};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use static_cell::StaticCell;

use crate::{
    app::{
        audio_mixer::AudioMixer,
        events::{
            AUDIO_BUFFER_HEADPHONES, AUDIO_BUFFER_SPEAKERS, AUDIO_BUFFER_TX, COMPRESSION,
            COMPRESSION_METER, NR_ENABLED, NR_LEVEL, SQUELCH, USB_AUDIO_TX, VOLUME,
        },
        tone_generator::ToneGenerator,
    },
    front_panel::events::{AUDIO_MIC_BUFFER, HEADPHONES_CONNECTED},
    main_board::events::{AUDIO_RX_BUFFER, CURRENT_RSSI2},
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
    let mut rx_rcv = AUDIO_RX_BUFFER.receiver().unwrap();
    let mut mic_rcv = AUDIO_MIC_BUFFER.receiver().unwrap();
    let mut usb_tx_rcv = USB_AUDIO_TX.receiver().unwrap();
    loop {
        let audio_rx = rx_rcv.changed().await;
        let mic = mic_rcv.changed().await;
        let generator = tone_generator.lock().await.next_buffer();

        let mut mixer = mutex.lock().await;
        mixer.set_buffer_rx(audio_rx);
        mixer.set_buffer_generator(generator);
        mixer.set_buffer_mic(mic);

        if let Some(usb_audio) = usb_tx_rcv.try_changed() {
            mixer.set_buffer_usb_tx(usb_audio);
        }

        mixer.mix();
        COMPRESSION_METER.sender().send(mixer.gain_reduction());

        AUDIO_BUFFER_TX.sender().send(mixer.get_buffer_tx());
        AUDIO_BUFFER_HEADPHONES.sender().send(mixer.get_buffer_headphones());
        AUDIO_BUFFER_SPEAKERS.sender().send(mixer.get_buffer_speakers());
    }
}

#[embassy_executor::task]
async fn controls_task(mutex: &'static Mutex<ThreadModeRawMutex, AudioMixer>) {
    let mut volume_rcv = VOLUME.receiver().unwrap();
    let mut hp_rcv = HEADPHONES_CONNECTED.receiver().unwrap();
    let mut squelch_rcv = SQUELCH.receiver().unwrap();
    let mut rssi_rcv = CURRENT_RSSI2.receiver().unwrap();
    let mut compression_rcv = COMPRESSION.receiver().unwrap();
    let mut nr_enabled_rcv = NR_ENABLED.receiver().unwrap();
    let mut nr_level_rcv = NR_LEVEL.receiver().unwrap();
    loop {
        match select(
            select5(
                volume_rcv.changed(),
                hp_rcv.changed(),
                squelch_rcv.changed(),
                rssi_rcv.changed(),
                compression_rcv.changed(),
            ),
            select(nr_enabled_rcv.changed(), nr_level_rcv.changed()),
        )
        .await
        {
            Either::First(Either5::First(volume)) => {
                mutex.lock().await.set_volume(volume);
            }
            Either::First(Either5::Second(connected)) => {
                mutex.lock().await.set_headphones_connected(connected);
            }
            Either::First(Either5::Third(squelch)) => {
                let dbm = squelch_to_dbm(squelch.raw());
                mutex.lock().await.set_squelch_threshold(dbm);
            }
            Either::First(Either5::Fourth(rssi)) => {
                mutex.lock().await.update_squelch(rssi.dbm);
            }
            Either::First(Either5::Fifth(compression)) => {
                mutex.lock().await.set_compression(compression);
            }
            Either::Second(Either::First(enabled)) => {
                mutex.lock().await.set_nr_enabled(enabled);
            }
            Either::Second(Either::Second(level)) => {
                mutex.lock().await.set_nr_level(level);
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
