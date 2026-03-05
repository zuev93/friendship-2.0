use druzhba_macros::instrumented;
use crate::runtime_stats::TaskId;
use embassy_executor::Spawner;
use embassy_futures::select::{select, select4, select5, Either, Either4, Either5};
use embassy_stm32::peripherals::FMAC;
use embassy_stm32::Peri;
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use static_cell::StaticCell;

use crate::{
    app::{
        audio_mixer::AudioMixer,
        cordic_math::CordicMutex,
        events::{
            ANF_ENABLED, AUDIO_AGC_MODE, AUDIO_BUFFER_HEADPHONES, AUDIO_BUFFER_SPEAKERS,
            AUDIO_BUFFER_TX, CLARIFIER_MODE, CLARIFIER_VALUE, COMPRESSION, COMPRESSION_METER,
            CW_PEAK_ENABLED, CW_PEAK_WIDTH, CW_PITCH, CW_SIDETONE_ACTIVE, DSP_BW,
            DSP_FILTER_ENABLED, DSP_PBT, NB_ENABLED, NB_LEVEL, NR_ENABLED, NR_LEVEL,
            RX_EQ_ENABLED, RX_EQ_HIGH, RX_EQ_LOW, RX_EQ_MID, SQUELCH, SWEEP_DATA, TRANSMIT_MODE,
            TX_EQ_ENABLED, TX_EQ_HIGH, TX_EQ_LOW, TX_EQ_MID, USB_AUDIO_TX, VOLUME,
            VOX_ANTI_TRIP, VOX_DELAY, VOX_ENABLED, VOX_GAIN, WATERFALL_LINE, WATERFALL_SPAN,
        },
        tasks::arbiters::{
            mode::{ModeCommand, MODE_CMD},
            scan::{ScanCommand, SCAN_CMD},
        },
        tone_generator::ToneGenerator,
        types::{ClarifierMode, TransmitMode, WaterfallLine, WATERFALL_BINS},
    },
    consts::ADC_BUFFER_SIZE,
    dsp::{types::FftResult, DspPipeline},
    front_panel::events::{AUDIO_MIC_BUFFER, HEADPHONES_CONNECTED},
    main_board::events::{AGC_DAC_VALUE, AUDIO_RX_BUFFER, CURRENT_RSSI2},
    main_board::types::RssiDbm,
};

pub fn spawn_tasks(
    spawner: Spawner,
    tone_generator: &'static Mutex<ThreadModeRawMutex, ToneGenerator>,
    cordic: &'static CordicMutex,
    fmac_peri: Peri<'static, FMAC>,
) {
    static MIXER: StaticCell<Mutex<ThreadModeRawMutex, AudioMixer>> = StaticCell::new();
    let mixer = MIXER.init(Mutex::new(AudioMixer::new(cordic)));

    static DSP: StaticCell<Mutex<ThreadModeRawMutex, DspPipeline>> = StaticCell::new();
    let dsp = DSP.init(Mutex::new(DspPipeline::new(cordic, fmac_peri)));

    spawner.must_spawn(audio_task(mixer, dsp, tone_generator));
    spawner.must_spawn(controls_task(mixer));
    spawner.must_spawn(dsp_controls_task(mixer));
    spawner.must_spawn(dsp_pipeline_controls_task(dsp));
    spawner.must_spawn(dsp_extra_controls_task(dsp));
    spawner.must_spawn(vox_controls_task(mixer));
}

const DEFAULT_SPAN_HZ: u32 = 100_000;
const IF_BW_HZ: u32 = 50_000;

#[instrumented(TaskId::Audio)]
#[embassy_executor::task]
async fn audio_task(
    mixer: &'static Mutex<ThreadModeRawMutex, AudioMixer>,
    dsp: &'static Mutex<ThreadModeRawMutex, DspPipeline>,
    tone_generator: &'static Mutex<ThreadModeRawMutex, ToneGenerator>,
) {
    let mut rx_rcv = AUDIO_RX_BUFFER.receiver().unwrap();
    let mut mic_rcv = AUDIO_MIC_BUFFER.receiver().unwrap();
    let mut usb_tx_rcv = USB_AUDIO_TX.receiver().unwrap();
    let mut sweep_rcv = SWEEP_DATA.anon_receiver();
    let mut span_rcv = WATERFALL_SPAN.anon_receiver();
    let mut sweep_cache = [0i8; WATERFALL_BINS];
    let mut current_span = DEFAULT_SPAN_HZ;
    loop {
        let adc_rx = rx_rcv.changed().await;
        let mic = mic_rcv.changed().await;
        let generator = tone_generator.lock().await.next_buffer();

        if let Some(span) = span_rcv.try_changed() {
            current_span = span.max(IF_BW_HZ);
        }
        if let Some(sweep_line) = sweep_rcv.try_changed() {
            sweep_cache = sweep_line.bins;
        }

        let (rx_decimated, fft_result) = {
            let mut pipeline = dsp.lock().await;
            let result = pipeline.process_rx_with_fft(&adc_rx);

            let peak_dbfs = adc_peak_dbfs(&adc_rx);
            let agc_dac = pipeline.process_adc_peak(peak_dbfs);
            AGC_DAC_VALUE.sender().send(agc_dac);

            let dbm = pipeline.smeter_dbm();
            let _s_units = pipeline.smeter_s_units();
            let _s_string = pipeline.smeter_s_string();
            let _agc_gain = pipeline.agc_current_gain();
            let _overload = pipeline.adc_overload();
            CURRENT_RSSI2.sender().send(RssiDbm { dbm: dbm as i8 });
            result
        };

        emit_composite_waterfall(&fft_result, &sweep_cache, current_span);

        let vox_transition;
        let tx_audio;
        {
            let mut mix = mixer.lock().await;
            mix.set_buffer_rx(rx_decimated);
            mix.set_buffer_generator(generator);
            mix.set_buffer_mic(mic);

            if let Some(usb_audio) = usb_tx_rcv.try_changed() {
                mix.set_buffer_usb_tx(usb_audio);
            }

            vox_transition = mix.process_vox();

            mix.mix();
            COMPRESSION_METER.sender().send(mix.gain_reduction());

            tx_audio = mix.get_buffer_tx();

            AUDIO_BUFFER_HEADPHONES
                .sender()
                .send(mix.get_buffer_headphones());
            AUDIO_BUFFER_SPEAKERS
                .sender()
                .send(mix.get_buffer_speakers());
        }

        {
            let mut tx_dac = [0u32; ADC_BUFFER_SIZE];
            let mut pipeline = dsp.lock().await;
            pipeline.process_tx(&tx_audio, &mut tx_dac);
            AUDIO_BUFFER_TX.sender().send(tx_dac);
        }

        if let Some(activate) = vox_transition {
            if activate {
                MODE_CMD.signal(ModeCommand::VoxActivate);
                SCAN_CMD.signal(ScanCommand::Stop);
            } else {
                MODE_CMD.signal(ModeCommand::VoxDeactivate);
            }
        }
    }
}

#[instrumented(TaskId::Controls)]
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

#[instrumented(TaskId::DspControls)]
#[embassy_executor::task]
async fn dsp_controls_task(mutex: &'static Mutex<ThreadModeRawMutex, AudioMixer>) {
    let mut anf_rcv = ANF_ENABLED.receiver().unwrap();
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
            select5(
                anf_rcv.changed(),
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
        )
        .await
        {
            Either::First(Either5::First(enabled)) => {
                mutex.lock().await.set_anf_enabled(enabled);
            }
            Either::First(Either5::Second(enabled)) => {
                mutex.lock().await.set_tx_eq_enabled(enabled);
            }
            Either::First(Either5::Third(gain)) => {
                mutex.lock().await.set_tx_eq_low(gain);
            }
            Either::First(Either5::Fourth(gain)) => {
                mutex.lock().await.set_tx_eq_mid(gain);
            }
            Either::First(Either5::Fifth(gain)) => {
                mutex.lock().await.set_tx_eq_high(gain);
            }
            Either::Second(Either4::First(enabled)) => {
                mutex.lock().await.set_rx_eq_enabled(enabled);
            }
            Either::Second(Either4::Second(gain)) => {
                mutex.lock().await.set_rx_eq_low(gain);
            }
            Either::Second(Either4::Third(gain)) => {
                mutex.lock().await.set_rx_eq_mid(gain);
            }
            Either::Second(Either4::Fourth(gain)) => {
                mutex.lock().await.set_rx_eq_high(gain);
            }
        }
    }
}

#[instrumented(TaskId::DspControls)]
#[embassy_executor::task]
async fn dsp_pipeline_controls_task(dsp: &'static Mutex<ThreadModeRawMutex, DspPipeline>) {
    use crate::app::events::DEMOD_MODE_OVERRIDE;
    use embassy_futures::select::select3;
    use embassy_futures::select::Either3;

    let mut dsp_filter_rcv = DSP_FILTER_ENABLED.receiver().unwrap();
    let mut dsp_bw_rcv = DSP_BW.receiver().unwrap();
    let mut dsp_pbt_rcv = DSP_PBT.receiver().unwrap();
    let mut transmit_mode_rcv = TRANSMIT_MODE.receiver().unwrap();
    let mut agc_mode_rcv = AUDIO_AGC_MODE.receiver().unwrap();
    let mut nb_enabled_rcv = NB_ENABLED.receiver().unwrap();
    let mut nb_level_rcv = NB_LEVEL.receiver().unwrap();
    let mut cw_peak_rcv = CW_PEAK_ENABLED.receiver().unwrap();
    let mut cw_peak_width_rcv = CW_PEAK_WIDTH.receiver().unwrap();
    let mut cw_pitch_rcv = CW_PITCH.receiver().unwrap();
    let mut demod_override_rcv = DEMOD_MODE_OVERRIDE.receiver().unwrap();

    loop {
        match select(
            select3(
                select4(
                    dsp_filter_rcv.changed(),
                    dsp_bw_rcv.changed(),
                    dsp_pbt_rcv.changed(),
                    transmit_mode_rcv.changed(),
                ),
                select4(
                    agc_mode_rcv.changed(),
                    nb_enabled_rcv.changed(),
                    nb_level_rcv.changed(),
                    cw_pitch_rcv.changed(),
                ),
                select(cw_peak_rcv.changed(), cw_peak_width_rcv.changed()),
            ),
            demod_override_rcv.changed(),
        )
        .await
        {
            Either::Second(idx) => {
                use crate::dsp::types::DemodMode;
                dsp.lock().await.set_demod_mode(DemodMode::from_index(idx));
            }
            Either::First(inner) => match inner {
                Either3::First(Either4::First(enabled)) => {
                    dsp.lock().await.set_filter_enabled(enabled);
                }
                Either3::First(Either4::Second(bw)) => {
                    dsp.lock().await.set_bandwidth(bw.raw() as f32);
                }
                Either3::First(Either4::Third(pbt)) => {
                    dsp.lock().await.set_shift(pbt.raw() as f32);
                }
                Either3::First(Either4::Fourth(mode)) => {
                    use crate::dsp::types::{AgcPreset, DemodMode};
                    let (demod, default_agc) = match mode {
                        TransmitMode::Usb => (DemodMode::Usb, AgcPreset::SsbFast),
                        TransmitMode::Lsb => (DemodMode::Lsb, AgcPreset::SsbFast),
                        TransmitMode::Cw => (DemodMode::Cw, AgcPreset::Cw),
                        TransmitMode::Am => (DemodMode::Am, AgcPreset::Am),
                    };
                    let mut p = dsp.lock().await;
                    p.set_demod_mode(demod);
                    p.set_agc_preset(default_agc);
                }
                Either3::Second(Either4::First(mode)) => {
                    use crate::app::types::AudioAgcMode;
                    use crate::dsp::types::AgcPreset;
                    let preset = match mode {
                        AudioAgcMode::Off => AgcPreset::Off,
                        AudioAgcMode::Slow => AgcPreset::SsbSlow,
                        AudioAgcMode::Med => AgcPreset::SsbFast,
                        AudioAgcMode::Fast => AgcPreset::SsbFast,
                    };
                    dsp.lock().await.set_agc_preset(preset);
                }
                Either3::Second(Either4::Second(enabled)) => {
                    dsp.lock().await.set_nb_enabled(enabled);
                }
                Either3::Second(Either4::Third(level)) => {
                    dsp.lock().await.set_nb_threshold(level.raw() as u8);
                }
                Either3::Second(Either4::Fourth(pitch)) => {
                    dsp.lock().await.set_cw_pitch(pitch.raw());
                }
                Either3::Third(Either::First(enabled)) => {
                    dsp.lock().await.set_cw_peak_enabled(enabled);
                }
                Either3::Third(Either::Second(width)) => {
                    dsp.lock().await.set_cw_peak_bw(width.raw() as f32);
                }
            },
        }
    }
}

#[instrumented(TaskId::VoxControls)]
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

fn adc_peak_dbfs(adc_buffer: &[u32; ADC_BUFFER_SIZE]) -> f32 {
    let mut max_abs: i32 = 0;
    let mono_samples = ADC_BUFFER_SIZE / 2;
    for frame in 0..mono_samples {
        let raw = adc_buffer[frame * 2];
        let signed_24 = ((raw << 8) as i32) >> 8;
        let abs = if signed_24 < 0 { -signed_24 } else { signed_24 };
        if abs > max_abs {
            max_abs = abs;
        }
    }
    if max_abs == 0 {
        return -120.0;
    }
    let normalized = max_abs as f32 / 8_388_607.0;
    20.0 * log2_approx(normalized) * 0.30103
}

fn log2_approx(x: f32) -> f32 {
    if x <= 0.0 {
        return -40.0;
    }
    let bits = x.to_bits();
    let exp = ((bits >> 23) & 0xFF) as f32 - 127.0;
    let mant = f32::from_bits((bits & 0x007F_FFFF) | 0x3F80_0000);
    exp + (mant - 1.0) * 1.4427
}

fn emit_composite_waterfall(fft: &FftResult, sweep_cache: &[i8; WATERFALL_BINS], span_hz: u32) {
    const IF_LOW_BIN: usize = 133;
    const IF_HIGH_BIN: usize = 400;
    const IF_FFT_SPAN: usize = IF_HIGH_BIN - IF_LOW_BIN;

    let live_width = if span_hz <= IF_BW_HZ {
        WATERFALL_BINS
    } else {
        ((IF_BW_HZ as usize * WATERFALL_BINS) / span_hz as usize).max(1)
    };
    let live_start = (WATERFALL_BINS - live_width) / 2;
    let live_end = live_start + live_width;

    let mut line = WaterfallLine::new();

    for i in 0..WATERFALL_BINS {
        line.bins[i] = sweep_cache[i];
    }

    for i in live_start..live_end {
        let fft_pos = i - live_start;
        let src_bin = IF_LOW_BIN + (fft_pos * IF_FFT_SPAN) / live_width;
        if src_bin < IF_HIGH_BIN {
            line.bins[i] = fft.bins[src_bin].clamp(-127.0, 127.0) as i8;
        }
    }

    line.live_start = live_start as u8;
    line.live_end = live_end as u8;
    line.complete = true;
    WATERFALL_LINE.sender().send(line);
}

#[instrumented(TaskId::DspControls)]
#[embassy_executor::task]
async fn dsp_extra_controls_task(dsp: &'static Mutex<ThreadModeRawMutex, DspPipeline>) {
    use crate::app::events::{FILTER, IF_GAIN};
    use crate::app::types::FilterType;
    use crate::dsp::types::FilterPreset;
    use embassy_futures::select::select5;
    use embassy_futures::select::Either5;

    let mut clarifier_mode_rcv = CLARIFIER_MODE.receiver().unwrap();
    let mut clarifier_val_rcv = CLARIFIER_VALUE.receiver().unwrap();
    let mut cw_sidetone_rcv = CW_SIDETONE_ACTIVE.receiver().unwrap();
    let mut if_gain_rcv = IF_GAIN.receiver().unwrap();
    let mut filter_type_rcv = FILTER.receiver().unwrap();

    let mut current_clarifier_mode = ClarifierMode::Off;

    loop {
        match select5(
            clarifier_mode_rcv.changed(),
            clarifier_val_rcv.changed(),
            cw_sidetone_rcv.changed(),
            if_gain_rcv.changed(),
            filter_type_rcv.changed(),
        )
        .await
        {
            Either5::First(mode) => {
                current_clarifier_mode = mode;
                if mode == ClarifierMode::Off {
                    dsp.lock().await.set_rit_offset(0);
                }
            }
            Either5::Second(val) => {
                if current_clarifier_mode == ClarifierMode::Rit {
                    dsp.lock().await.set_rit_offset(val.raw() as i32);
                }
            }
            Either5::Third(active) => {
                dsp.lock().await.set_cw_key(active);
            }
            Either5::Fourth(gain) => {
                let gain_db = gain.raw() as f32 * 80.0 / 1000.0;
                dsp.lock().await.set_agc_manual_gain(gain_db);
            }
            Either5::Fifth(filter_type) => {
                let preset_idx: u8 = match filter_type {
                    FilterType::Wide => 3,
                    FilterType::Medium => 2,
                    FilterType::Narrow => 2,
                    FilterType::CwNarrow => 0,
                };
                let preset = FilterPreset::by_index(preset_idx);
                dsp.lock().await.apply_filter_preset(&preset);
            }
        }
    }
}
