use embassy_executor::Spawner;
use embassy_futures::select::{select, select4, select5, Either, Either4, Either5};
use embassy_stm32::peripherals::{CORDIC, FMAC};
use embassy_stm32::Peri;
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use static_cell::StaticCell;

use crate::{
    app::{
        audio_mixer::AudioMixer,
        events::{
            ANF_ENABLED, AUDIO_AGC_MODE, AUDIO_BUFFER_HEADPHONES, AUDIO_BUFFER_SPEAKERS,
            AUDIO_BUFFER_TX, COMPRESSION, COMPRESSION_METER, CW_PEAK_ENABLED, CW_PEAK_WIDTH,
            CW_PITCH, DSP_BW, DSP_FILTER_ENABLED, DSP_PBT, NR_ENABLED, NR_LEVEL, RX_EQ_ENABLED,
            RX_EQ_HIGH, RX_EQ_LOW, RX_EQ_MID, SQUELCH, TRANSMIT_MODE, TX_EQ_ENABLED, TX_EQ_HIGH,
            TX_EQ_LOW, TX_EQ_MID, USB_AUDIO_TX, VOLUME, VOX_ANTI_TRIP, VOX_DELAY, VOX_ENABLED,
            VOX_GAIN,
        },
        tasks::arbiters::mode::{ModeCommand, MODE_CMD},
        tone_generator::ToneGenerator,
        types::TransmitMode,
    },
    front_panel::events::{AUDIO_MIC_BUFFER, HEADPHONES_CONNECTED},
    main_board::events::{AUDIO_RX_BUFFER, CURRENT_RSSI2},
};

pub fn spawn_tasks(
    spawner: Spawner,
    tone_generator: &'static Mutex<ThreadModeRawMutex, ToneGenerator>,
    cordic_peri: Peri<'static, CORDIC>,
    fmac_peri: Peri<'static, FMAC>,
) {
    static MIXER: StaticCell<Mutex<ThreadModeRawMutex, AudioMixer>> = StaticCell::new();
    let mixer = MIXER.init(Mutex::new(AudioMixer::new(cordic_peri, fmac_peri)));
    spawner.must_spawn(audio_task(mixer, tone_generator));
    spawner.must_spawn(controls_task(mixer));
    spawner.must_spawn(dsp_controls_task(mixer));
    spawner.must_spawn(vox_controls_task(mixer));
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

        let vox_transition;
        {
            let mut mixer = mutex.lock().await;
            mixer.set_buffer_rx(audio_rx);
            mixer.set_buffer_generator(generator);
            mixer.set_buffer_mic(mic);

            if let Some(usb_audio) = usb_tx_rcv.try_changed() {
                mixer.set_buffer_usb_tx(usb_audio);
            }

            vox_transition = mixer.process_vox();

            mixer.mix();
            COMPRESSION_METER.sender().send(mixer.gain_reduction());

            AUDIO_BUFFER_TX.sender().send(mixer.get_buffer_tx());
            AUDIO_BUFFER_HEADPHONES
                .sender()
                .send(mixer.get_buffer_headphones());
            AUDIO_BUFFER_SPEAKERS
                .sender()
                .send(mixer.get_buffer_speakers());
        }

        if let Some(activate) = vox_transition {
            if activate {
                MODE_CMD.signal(ModeCommand::VoxActivate);
            } else {
                MODE_CMD.signal(ModeCommand::VoxDeactivate);
            }
        }
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

#[embassy_executor::task]
async fn dsp_controls_task(mutex: &'static Mutex<ThreadModeRawMutex, AudioMixer>) {
    let mut anf_rcv = ANF_ENABLED.receiver().unwrap();
    let mut dsp_filter_rcv = DSP_FILTER_ENABLED.receiver().unwrap();
    let mut dsp_bw_rcv = DSP_BW.receiver().unwrap();
    let mut dsp_pbt_rcv = DSP_PBT.receiver().unwrap();
    let mut cw_peak_rcv = CW_PEAK_ENABLED.receiver().unwrap();
    let mut cw_peak_width_rcv = CW_PEAK_WIDTH.receiver().unwrap();
    let mut cw_pitch_rcv = CW_PITCH.receiver().unwrap();
    let mut agc_mode_rcv = AUDIO_AGC_MODE.receiver().unwrap();
    let mut tx_eq_en_rcv = TX_EQ_ENABLED.receiver().unwrap();
    let mut tx_eq_low_rcv = TX_EQ_LOW.receiver().unwrap();
    let mut tx_eq_mid_rcv = TX_EQ_MID.receiver().unwrap();
    let mut tx_eq_high_rcv = TX_EQ_HIGH.receiver().unwrap();
    let mut rx_eq_en_rcv = RX_EQ_ENABLED.receiver().unwrap();
    let mut rx_eq_low_rcv = RX_EQ_LOW.receiver().unwrap();
    let mut rx_eq_mid_rcv = RX_EQ_MID.receiver().unwrap();
    let mut rx_eq_high_rcv = RX_EQ_HIGH.receiver().unwrap();

    loop {
        match select(
            select(
                select4(
                    anf_rcv.changed(),
                    dsp_filter_rcv.changed(),
                    dsp_bw_rcv.changed(),
                    dsp_pbt_rcv.changed(),
                ),
                select4(
                    cw_peak_rcv.changed(),
                    cw_peak_width_rcv.changed(),
                    cw_pitch_rcv.changed(),
                    agc_mode_rcv.changed(),
                ),
            ),
            select(
                select4(
                    tx_eq_en_rcv.changed(),
                    tx_eq_low_rcv.changed(),
                    tx_eq_mid_rcv.changed(),
                    tx_eq_high_rcv.changed(),
                ),
                select4(
                    rx_eq_en_rcv.changed(),
                    rx_eq_low_rcv.changed(),
                    rx_eq_mid_rcv.changed(),
                    rx_eq_high_rcv.changed(),
                ),
            ),
        )
        .await
        {
            Either::First(Either::First(Either4::First(enabled))) => {
                mutex.lock().await.set_anf_enabled(enabled);
            }
            Either::First(Either::First(Either4::Second(enabled))) => {
                mutex.lock().await.set_dsp_filter_enabled(enabled);
            }
            Either::First(Either::First(Either4::Third(bw))) => {
                mutex.lock().await.set_dsp_bandwidth(bw.raw());
            }
            Either::First(Either::First(Either4::Fourth(pbt))) => {
                mutex.lock().await.set_dsp_shift(pbt.raw());
            }
            Either::First(Either::Second(Either4::First(enabled))) => {
                mutex.lock().await.set_cw_peak_enabled(enabled);
            }
            Either::First(Either::Second(Either4::Second(width))) => {
                mutex.lock().await.set_cw_peak_width(width.raw());
            }
            Either::First(Either::Second(Either4::Third(pitch))) => {
                mutex.lock().await.set_cw_pitch(pitch.raw());
            }
            Either::First(Either::Second(Either4::Fourth(mode))) => {
                mutex.lock().await.set_audio_agc_mode(mode);
            }
            Either::Second(Either::First(Either4::First(enabled))) => {
                mutex.lock().await.set_tx_eq_enabled(enabled);
            }
            Either::Second(Either::First(Either4::Second(gain))) => {
                mutex.lock().await.set_tx_eq_low(gain);
            }
            Either::Second(Either::First(Either4::Third(gain))) => {
                mutex.lock().await.set_tx_eq_mid(gain);
            }
            Either::Second(Either::First(Either4::Fourth(gain))) => {
                mutex.lock().await.set_tx_eq_high(gain);
            }
            Either::Second(Either::Second(Either4::First(enabled))) => {
                mutex.lock().await.set_rx_eq_enabled(enabled);
            }
            Either::Second(Either::Second(Either4::Second(gain))) => {
                mutex.lock().await.set_rx_eq_low(gain);
            }
            Either::Second(Either::Second(Either4::Third(gain))) => {
                mutex.lock().await.set_rx_eq_mid(gain);
            }
            Either::Second(Either::Second(Either4::Fourth(gain))) => {
                mutex.lock().await.set_rx_eq_high(gain);
            }
        }
    }
}

#[embassy_executor::task]
async fn vox_controls_task(mutex: &'static Mutex<ThreadModeRawMutex, AudioMixer>) {
    let mut vox_enabled_rcv = VOX_ENABLED.receiver().unwrap();
    let mut vox_gain_rcv = VOX_GAIN.receiver().unwrap();
    let mut vox_delay_rcv = VOX_DELAY.receiver().unwrap();
    let mut vox_anti_trip_rcv = VOX_ANTI_TRIP.receiver().unwrap();
    let mut transmit_mode_rcv = TRANSMIT_MODE.receiver().unwrap();

    loop {
        match select(
            select4(
                vox_enabled_rcv.changed(),
                vox_gain_rcv.changed(),
                vox_delay_rcv.changed(),
                vox_anti_trip_rcv.changed(),
            ),
            transmit_mode_rcv.changed(),
        )
        .await
        {
            Either::First(Either4::First(enabled)) => {
                mutex.lock().await.set_vox_enabled(enabled);
            }
            Either::First(Either4::Second(gain)) => {
                mutex.lock().await.set_vox_gain(gain.raw() as u16);
            }
            Either::First(Either4::Third(delay)) => {
                mutex.lock().await.set_vox_delay(delay.ms());
            }
            Either::First(Either4::Fourth(anti_trip)) => {
                mutex.lock().await.set_vox_anti_trip(anti_trip.raw() as u16);
            }
            Either::Second(mode) => {
                let voice = matches!(mode, TransmitMode::Usb | TransmitMode::Lsb);
                mutex.lock().await.set_vox_voice_mode(voice);
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
