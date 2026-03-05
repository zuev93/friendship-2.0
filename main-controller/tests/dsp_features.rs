use std::cell::RefCell;
use std::path::PathBuf;

use embassy_sync::blocking_mutex::Mutex;

use druzhba_main_controller::audio_mixer::AudioMixer;
use druzhba_main_controller::consts::{ADC_BUFFER_SIZE, AUDIO_BUFFER_SIZE};
use druzhba_main_controller::cordic_math::{CordicMath, CordicMutex};
use druzhba_main_controller::dsp::types::{AgcPreset, DemodMode, DSP_BLOCK_SIZE};
use druzhba_main_controller::dsp::DspPipeline;
use druzhba_main_controller::mixer_types::{Compression, EqGain, NrLevel, Volume};

fn output_dir(subdir: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test-output")
        .join(subdir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-fixtures")
}

fn make_cordic() -> &'static CordicMutex {
    Box::leak(Box::new(Mutex::new(RefCell::new(CordicMath::new()))))
}

fn write_wav(path: &std::path::Path, samples: &[i16], sample_rate: u32) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).unwrap();
    for &s in samples {
        writer.write_sample(s).unwrap();
    }
    writer.finalize().unwrap();
}

fn load_wav_i16(path: &std::path::Path) -> Vec<i16> {
    let mut reader = hound::WavReader::open(path).unwrap();
    reader.samples::<i16>().map(|s| s.unwrap()).collect()
}

fn i16_to_u16(samples: &[i16]) -> Vec<u16> {
    samples.iter().map(|&s| (s as i32 + 32768) as u16).collect()
}

fn u16_to_i16(samples: &[u16]) -> Vec<i16> {
    samples.iter().map(|&s| (s as i32 - 32768) as i16).collect()
}

fn rms_i16(samples: &[i16]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f64 = samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
    libm::sqrt(sum_sq / samples.len() as f64) as f32
}

fn rms_f32(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f64 = samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
    libm::sqrt(sum_sq / samples.len() as f64) as f32
}

fn generate_tone_i16(
    freq_hz: f32,
    sample_rate: f32,
    num_samples: usize,
    amplitude: f32,
) -> Vec<i16> {
    let mut samples = Vec::with_capacity(num_samples);
    for i in 0..num_samples {
        let t = i as f32 / sample_rate;
        let val = amplitude * libm::sinf(2.0 * core::f32::consts::PI * freq_hz * t);
        samples.push((val * 32767.0).clamp(-32768.0, 32767.0) as i16);
    }
    samples
}

fn to_blocks_u16(samples: &[u16]) -> Vec<[u16; AUDIO_BUFFER_SIZE]> {
    let num_blocks = samples.len() / AUDIO_BUFFER_SIZE;
    let mut blocks = Vec::with_capacity(num_blocks);
    for b in 0..num_blocks {
        let start = b * AUDIO_BUFFER_SIZE;
        let mut buf = [0u16; AUDIO_BUFFER_SIZE];
        buf.copy_from_slice(&samples[start..start + AUDIO_BUFFER_SIZE]);
        blocks.push(buf);
    }
    blocks
}

fn tx_modulate(
    cordic: &'static CordicMutex,
    mode: DemodMode,
    audio_blocks: &[[u16; AUDIO_BUFFER_SIZE]],
) -> Vec<[u32; ADC_BUFFER_SIZE]> {
    let mut dsp_tx = DspPipeline::new(cordic);
    dsp_tx.set_demod_mode(mode);
    dsp_tx.set_filter_enabled(true);
    dsp_tx.set_compressor_enabled(false);

    let mut if_buffers: Vec<[u32; ADC_BUFFER_SIZE]> = Vec::with_capacity(audio_blocks.len());
    for audio_block in audio_blocks {
        let mut dac_out = [0u32; ADC_BUFFER_SIZE];
        dsp_tx.process_tx(audio_block, &mut dac_out);
        if_buffers.push(dac_out);
    }
    if_buffers
}

fn rx_process(
    cordic: &'static CordicMutex,
    mode: DemodMode,
    if_buffers: &[[u32; ADC_BUFFER_SIZE]],
    setup: impl FnOnce(&mut DspPipeline),
) -> Vec<f32> {
    let mut dsp_rx = DspPipeline::new(cordic);
    dsp_rx.set_demod_mode(mode);
    dsp_rx.set_filter_enabled(true);
    dsp_rx.set_agc_preset(AgcPreset::Off);
    setup(&mut dsp_rx);

    let mut raw: Vec<f32> = Vec::with_capacity(if_buffers.len() * DSP_BLOCK_SIZE);
    for if_buf in if_buffers {
        let block = dsp_rx.process_rx_raw(if_buf);
        raw.extend_from_slice(&block);
    }
    raw
}

fn rx_process_with_agc(
    cordic: &'static CordicMutex,
    mode: DemodMode,
    if_buffers: &[[u32; ADC_BUFFER_SIZE]],
    agc_preset: AgcPreset,
) -> Vec<f32> {
    let mut dsp_rx = DspPipeline::new(cordic);
    dsp_rx.set_demod_mode(mode);
    dsp_rx.set_filter_enabled(true);
    dsp_rx.set_agc_preset(agc_preset);

    let mut raw: Vec<f32> = Vec::with_capacity(if_buffers.len() * DSP_BLOCK_SIZE);
    for if_buf in if_buffers {
        let block = dsp_rx.process_rx_raw(if_buf);
        raw.extend_from_slice(&block);
    }
    raw
}

fn normalize_to_i16(raw: &[f32], skip: usize) -> Vec<i16> {
    let peak = raw[skip..].iter().fold(0.0f32, |mx, &s| mx.max(s.abs()));
    let scale = if peak > 1e-6 { 1.0 / peak } else { 1.0 };
    raw.iter()
        .map(|&s| (s * scale * 32767.0).clamp(-32768.0, 32767.0) as i16)
        .collect()
}

fn mixer_process_blocks(mixer: &mut AudioMixer, rx_u16: &[u16]) -> Vec<u16> {
    let num_blocks = rx_u16.len() / AUDIO_BUFFER_SIZE;
    let mut output = Vec::with_capacity(rx_u16.len());
    let zero = [0u16; AUDIO_BUFFER_SIZE];

    for block in 0..num_blocks {
        let start = block * AUDIO_BUFFER_SIZE;
        let mut buf = [0u16; AUDIO_BUFFER_SIZE];
        buf.copy_from_slice(&rx_u16[start..start + AUDIO_BUFFER_SIZE]);
        mixer.set_buffer_rx(buf);
        mixer.set_buffer_mic(zero);
        mixer.set_buffer_generator(zero);
        mixer.mix();
        output.extend_from_slice(&mixer.get_buffer_headphones());
    }

    output
}

fn snr_db(signal_rms: f32, noise_rms: f32) -> f32 {
    if noise_rms < 1e-6 {
        return 99.0;
    }
    20.0 * libm::log10f(signal_rms / noise_rms)
}

fn score_0_100(value: f32, bad: f32, good: f32) -> u8 {
    if good > bad {
        ((value - bad) / (good - bad) * 100.0).clamp(0.0, 100.0) as u8
    } else {
        ((bad - value) / (bad - good) * 100.0).clamp(0.0, 100.0) as u8
    }
}

const SKIP: usize = AUDIO_BUFFER_SIZE * 4;

// ============================================================
// Test 1: Noise Blanker (pipeline: TX noisy speech → RX)
// ============================================================
#[test]
fn test_noise_blanker() {
    let cordic = make_cordic();
    let noisy_i16 = load_wav_i16(&fixtures_dir().join("speech-impulse-noise.wav"));
    let noisy_u16 = i16_to_u16(&noisy_i16);
    let noisy_blocks = to_blocks_u16(&noisy_u16);

    let clean_i16 = load_wav_i16(&fixtures_dir().join("speech.wav"));
    let clean_u16 = i16_to_u16(&clean_i16);
    let clean_blocks = to_blocks_u16(&clean_u16);
    let out = output_dir("nb");

    let if_noisy = tx_modulate(cordic, DemodMode::Usb, &noisy_blocks);
    let if_clean = tx_modulate(cordic, DemodMode::Usb, &clean_blocks);

    let raw_clean = rx_process(cordic, DemodMode::Usb, &if_clean, |_| {});

    for nb_enabled in [false, true] {
        let raw = rx_process(cordic, DemodMode::Usb, &if_noisy, |dsp| {
            dsp.set_nb_enabled(nb_enabled);
            if nb_enabled {
                dsp.set_nb_threshold(4);
            }
        });
        let output_i16 = normalize_to_i16(&raw, SKIP);

        let suffix = if nb_enabled { "nb-on" } else { "nb-off" };
        write_wav(
            &out.join(format!("output-{suffix}.wav")),
            &output_i16,
            48000,
        );

        let rms = rms_i16(&output_i16[SKIP..]);
        let peak: f32 = output_i16[SKIP..]
            .iter()
            .map(|s| s.saturating_abs())
            .max()
            .unwrap_or(0) as f32;
        let crest_db = 20.0 * libm::log10f(peak / rms.max(1.0));
        let rms_diff = rms_f32(
            &raw[SKIP..]
                .iter()
                .zip(raw_clean[SKIP..].iter())
                .map(|(&a, &b)| a - b)
                .collect::<Vec<_>>(),
        );
        let rms_clean_raw = rms_f32(&raw_clean[SKIP..]);
        let snr = snr_db(rms_clean_raw, rms_diff);
        let sc = score_0_100(snr, 0.0, 30.0);
        eprintln!(
            "[NB] {suffix}: rms={rms:.0} crest={crest_db:.1}dB snr={snr:.1}dB score={sc}/100"
        );
    }
}

// ============================================================
// Test 2: Noise Reduction (LMS NR in AudioMixer)
// ============================================================
#[test]
fn test_noise_reduction() {
    let cordic = make_cordic();
    let clean_i16 = load_wav_i16(&fixtures_dir().join("speech.wav"));
    let noisy_i16 = load_wav_i16(&fixtures_dir().join("speech-white-noise.wav"));
    let noisy_u16 = i16_to_u16(&noisy_i16);
    let out = output_dir("nr");

    for nr_enabled in [false, true] {
        let mut mixer = AudioMixer::new(cordic);
        mixer.set_nr_enabled(nr_enabled);
        if nr_enabled {
            mixer.set_nr_level(NrLevel::new(500));
        }

        let output = mixer_process_blocks(&mut mixer, &noisy_u16);
        let output_i16 = u16_to_i16(&output);

        let suffix = if nr_enabled { "nr-on" } else { "nr-off" };
        write_wav(
            &out.join(format!("output-{suffix}.wav")),
            &output_i16,
            48000,
        );

        let out_rms = rms_i16(&output_i16[SKIP..]);
        let noise_est: Vec<f32> = output_i16[SKIP..]
            .iter()
            .zip(clean_i16[SKIP..output_i16.len()].iter())
            .map(|(&a, &b)| (a - b) as f32)
            .collect();
        let noise_rms = rms_f32(&noise_est);
        let snr = snr_db(out_rms as f32, noise_rms);
        let sc = score_0_100(snr, 0.0, 20.0);
        eprintln!(
            "[NR] {suffix}: rms={out_rms:.0} noise_rms={noise_rms:.0} snr={snr:.1}dB score={sc}/100"
        );
    }
}

// ============================================================
// Test 3: Auto Notch Filter (ANF in AudioMixer)
// ============================================================
#[test]
fn test_auto_notch_filter() {
    let cordic = make_cordic();
    let clean_i16 = load_wav_i16(&fixtures_dir().join("speech.wav"));
    let noisy_i16 = load_wav_i16(&fixtures_dir().join("speech-tone-2khz.wav"));
    let noisy_u16 = i16_to_u16(&noisy_i16);
    let out = output_dir("anf");

    let tone_rms_in: f32 = {
        let len = clean_i16.len().min(noisy_i16.len());
        let diff: Vec<f32> = clean_i16[SKIP..len]
            .iter()
            .zip(noisy_i16[SKIP..len].iter())
            .map(|(&a, &b)| (b - a) as f32)
            .collect();
        rms_f32(&diff)
    };

    for anf_enabled in [false, true] {
        let mut mixer = AudioMixer::new(cordic);
        mixer.set_anf_enabled(anf_enabled);

        let output = mixer_process_blocks(&mut mixer, &noisy_u16);
        let output_i16 = u16_to_i16(&output);

        let suffix = if anf_enabled { "anf-on" } else { "anf-off" };
        write_wav(
            &out.join(format!("output-{suffix}.wav")),
            &output_i16,
            48000,
        );

        let len = clean_i16.len().min(output_i16.len());
        let skip_anf = AUDIO_BUFFER_SIZE * 16;
        let residual_tone: Vec<f32> = clean_i16[skip_anf..len]
            .iter()
            .zip(output_i16[skip_anf..len].iter())
            .map(|(&a, &b)| (b - a) as f32)
            .collect();
        let tone_rms_out = rms_f32(&residual_tone);
        let reduction_db = 20.0 * libm::log10f(tone_rms_in / tone_rms_out.max(1.0));
        let sc = score_0_100(reduction_db, 0.0, 20.0);
        eprintln!(
            "[ANF] {suffix}: tone_in={tone_rms_in:.0} tone_out={tone_rms_out:.0} reduction={reduction_db:.1}dB score={sc}/100"
        );
    }
}

// ============================================================
// Test 4: Squelch
// ============================================================
#[test]
fn test_squelch() {
    let cordic = make_cordic();
    let speech_i16 = load_wav_i16(&fixtures_dir().join("speech.wav"));
    let out = output_dir("squelch");

    let block_samples = AUDIO_BUFFER_SIZE;
    let num_blocks = speech_i16.len() / block_samples;
    let speech_u16 = i16_to_u16(&speech_i16);

    let mut mixer = AudioMixer::new(cordic);
    mixer.set_squelch_threshold(-60);

    let zero = [0u16; AUDIO_BUFFER_SIZE];
    let mut output = Vec::with_capacity(speech_i16.len());

    for block in 0..num_blocks {
        let start = block * block_samples;
        let mut buf = [0u16; AUDIO_BUFFER_SIZE];
        buf.copy_from_slice(&speech_u16[start..start + block_samples]);

        let third = num_blocks / 3;
        let rssi_dbm: i8 = if block < third || block >= 2 * third {
            -40
        } else {
            -80
        };
        mixer.update_squelch(rssi_dbm);

        mixer.set_buffer_rx(buf);
        mixer.set_buffer_mic(zero);
        mixer.set_buffer_generator(zero);
        mixer.mix();
        output.extend_from_slice(&mixer.get_buffer_headphones());
    }

    let output_i16 = u16_to_i16(&output);
    write_wav(&out.join("output-squelched.wav"), &output_i16, 48000);

    let mut silent_blocks = 0u32;
    let mut total_blocks = 0u32;
    for block in 0..num_blocks {
        let start = block * block_samples;
        let block_rms = rms_i16(&output_i16[start..start + block_samples]);
        total_blocks += 1;
        if block_rms < 10.0 {
            silent_blocks += 1;
        }
    }
    let pct = silent_blocks * 100 / total_blocks;
    assert!(
        silent_blocks > 0,
        "squelch should silence at least some blocks"
    );
    let sc = score_0_100(pct as f32, 0.0, 50.0);
    eprintln!("[SQUELCH] {silent_blocks}/{total_blocks} blocks silenced ({pct}%) score={sc}/100");
}

// ============================================================
// Test 5: AGC (pipeline: TX→RX with AGC presets)
// ============================================================
#[test]
fn test_agc() {
    let cordic = make_cordic();
    let varying_i16 = load_wav_i16(&fixtures_dir().join("speech-varying-level.wav"));
    let varying_u16 = i16_to_u16(&varying_i16);
    let blocks = to_blocks_u16(&varying_u16);
    let out = output_dir("agc");

    let if_buffers = tx_modulate(cordic, DemodMode::Usb, &blocks);

    for &preset in &[AgcPreset::Off, AgcPreset::SsbFast, AgcPreset::SsbSlow] {
        let preset_name = match preset {
            AgcPreset::Off => "off",
            AgcPreset::SsbFast => "fast",
            AgcPreset::SsbSlow => "slow",
            _ => "other",
        };

        let raw = rx_process_with_agc(cordic, DemodMode::Usb, &if_buffers, preset);
        let output_i16 = normalize_to_i16(&raw, SKIP);

        write_wav(
            &out.join(format!("output-agc-{preset_name}.wav")),
            &output_i16,
            48000,
        );

        let third = output_i16.len() / 3;
        let rms1 = rms_i16(&output_i16[SKIP..third]);
        let rms2 = rms_i16(&output_i16[third..2 * third]);
        let rms3 = rms_i16(&output_i16[2 * third..]);
        let ratio = if rms2 > 0.0 {
            (rms1 as f32 / rms2 as f32 + rms3 as f32 / rms2 as f32) / 2.0
        } else {
            0.0
        };
        let leveling_pct = (ratio * 100.0).min(100.0) as u8;
        eprintln!(
            "[AGC] {preset_name}: quiet1={rms1:.0} loud={rms2:.0} quiet2={rms3:.0} leveling={leveling_pct}%"
        );
    }
}

// ============================================================
// Test 6: RX EQ (AudioMixer biquad)
// ============================================================
#[test]
fn test_eq() {
    let cordic = make_cordic();
    let out = output_dir("eq");

    let num_samples = 48000;

    for &(freq, label) in &[(300.0f32, "300hz"), (1000.0, "1000hz"), (3000.0, "3000hz")] {
        let tone = generate_tone_i16(freq, 48000.0, num_samples, 0.5);
        let tone_u16 = i16_to_u16(&tone);

        let mut mixer_flat = AudioMixer::new(cordic);
        mixer_flat.set_rx_eq_enabled(false);
        let flat_out = mixer_process_blocks(&mut mixer_flat, &tone_u16);
        let flat_i16 = u16_to_i16(&flat_out);

        let mut mixer_boost = AudioMixer::new(cordic);
        mixer_boost.set_rx_eq_enabled(true);
        mixer_boost.set_rx_eq_low(EqGain::new(if freq < 500.0 { 12 } else { 0 }));
        mixer_boost.set_rx_eq_mid(EqGain::new(if freq >= 500.0 && freq <= 2000.0 {
            12
        } else {
            0
        }));
        mixer_boost.set_rx_eq_high(EqGain::new(if freq > 2000.0 { 12 } else { 0 }));
        let boost_out = mixer_process_blocks(&mut mixer_boost, &tone_u16);
        let boost_i16 = u16_to_i16(&boost_out);

        let mut mixer_cut = AudioMixer::new(cordic);
        mixer_cut.set_rx_eq_enabled(true);
        mixer_cut.set_rx_eq_low(EqGain::new(if freq < 500.0 { -12 } else { 0 }));
        mixer_cut.set_rx_eq_mid(EqGain::new(if freq >= 500.0 && freq <= 2000.0 {
            -12
        } else {
            0
        }));
        mixer_cut.set_rx_eq_high(EqGain::new(if freq > 2000.0 { -12 } else { 0 }));
        let cut_out = mixer_process_blocks(&mut mixer_cut, &tone_u16);
        let cut_i16 = u16_to_i16(&cut_out);

        write_wav(&out.join(format!("{label}-flat.wav")), &flat_i16, 48000);
        write_wav(&out.join(format!("{label}-boost.wav")), &boost_i16, 48000);
        write_wav(&out.join(format!("{label}-cut.wav")), &cut_i16, 48000);

        let skip = AUDIO_BUFFER_SIZE * 2;
        let rms_flat = rms_i16(&flat_i16[skip..]);
        let rms_boost = rms_i16(&boost_i16[skip..]);
        let rms_cut = rms_i16(&cut_i16[skip..]);
        let boost_db = 20.0 * libm::log10f(rms_boost / rms_flat.max(1.0));
        let cut_db = 20.0 * libm::log10f(rms_cut / rms_flat.max(1.0));
        eprintln!("[EQ] {label}: flat={rms_flat:.0} boost={rms_boost:.0}(+{boost_db:.1}dB) cut={rms_cut:.0}({cut_db:.1}dB)");

        assert!(
            rms_boost > rms_flat,
            "EQ {label}: boost ({rms_boost:.0}) should be louder than flat ({rms_flat:.0})"
        );
        assert!(
            rms_cut < rms_flat,
            "EQ {label}: cut ({rms_cut:.0}) should be quieter than flat ({rms_flat:.0})"
        );
    }
}

// ============================================================
// Test 7: TX Compressor (in AudioMixer)
// ============================================================
#[test]
fn test_tx_compressor() {
    let cordic = make_cordic();
    let speech_i16 = load_wav_i16(&fixtures_dir().join("speech.wav"));
    let speech_u16 = i16_to_u16(&speech_i16);
    let out = output_dir("compressor");

    let mut results: Vec<(i16, f32, i16)> = Vec::new();

    for &comp_level in &[0i16, 500, 1000] {
        let mut mixer = AudioMixer::new(cordic);
        mixer.set_compression(Compression::new(comp_level));

        let num_blocks = speech_u16.len() / AUDIO_BUFFER_SIZE;
        let zero = [0u16; AUDIO_BUFFER_SIZE];
        let mut output = Vec::with_capacity(speech_u16.len());

        for block in 0..num_blocks {
            let start = block * AUDIO_BUFFER_SIZE;
            let mut mic_buf = [0u16; AUDIO_BUFFER_SIZE];
            mic_buf.copy_from_slice(&speech_u16[start..start + AUDIO_BUFFER_SIZE]);
            mixer.set_buffer_mic(mic_buf);
            mixer.set_buffer_rx(zero);
            mixer.set_buffer_generator(zero);
            mixer.mix();
            output.extend_from_slice(&mixer.get_buffer_tx());
        }

        let output_i16 = u16_to_i16(&output);
        write_wav(
            &out.join(format!("output-comp-{comp_level}.wav")),
            &output_i16,
            48000,
        );

        let rms = rms_i16(&output_i16[SKIP..]);
        let peak: i16 = output_i16[SKIP..]
            .iter()
            .map(|s| s.saturating_abs())
            .max()
            .unwrap_or(0);
        let crest_db = 20.0 * libm::log10f(peak as f32 / rms.max(1.0));
        results.push((comp_level, rms, peak));
        eprintln!(
            "[COMPRESSOR] level={comp_level}: rms={rms:.0} peak={peak} crest={crest_db:.1}dB"
        );
    }

    let (_, rms0, _) = results[0];
    let (_, rms1000, _) = results[2];
    let compression_db = 20.0 * libm::log10f(rms0 / rms1000.max(1.0));
    eprintln!("[COMPRESSOR] effect: {compression_db:.1}dB dynamic range reduction");
}

// ============================================================
// Test 8: CW Peak Filter (pipeline: TX→RX with CW peak)
// ============================================================
#[test]
fn test_cw_peak_filter() {
    let cordic = make_cordic();
    let out = output_dir("cw-peak");

    let noisy_i16 = load_wav_i16(&fixtures_dir().join("cw-noisy-700hz.wav"));
    let noisy_u16 = i16_to_u16(&noisy_i16);
    let blocks = to_blocks_u16(&noisy_u16);

    let if_buffers = tx_modulate(cordic, DemodMode::Usb, &blocks);

    for peak_enabled in [false, true] {
        let raw = rx_process(cordic, DemodMode::Usb, &if_buffers, |dsp| {
            if peak_enabled {
                dsp.set_cw_peak_enabled(true);
                dsp.set_cw_pitch(700);
                dsp.set_cw_peak_bw(200.0);
            }
        });
        let output_i16 = normalize_to_i16(&raw, SKIP);

        let suffix = if peak_enabled { "peak-on" } else { "peak-off" };
        write_wav(
            &out.join(format!("output-{suffix}.wav")),
            &output_i16,
            48000,
        );

        let rms_total = rms_i16(&output_i16[SKIP..]);

        let n = output_i16[SKIP..].len() as f64;
        let mut sum_sin = 0.0f64;
        let mut sum_cos = 0.0f64;
        for (i, &s) in output_i16[SKIP..].iter().enumerate() {
            let t = (i + SKIP) as f64 / 48000.0;
            let sf = s as f64 / 32767.0;
            sum_sin += sf * libm::sin(2.0 * core::f64::consts::PI * 700.0 * t);
            sum_cos += sf * libm::cos(2.0 * core::f64::consts::PI * 700.0 * t);
        }
        let tone_rms = libm::sqrt(2.0 * (sum_sin * sum_sin + sum_cos * sum_cos) / (n * n)) as f32;
        let total_rms = rms_total as f32 / 32767.0;
        let noise_rms = libm::sqrtf((total_rms * total_rms - tone_rms * tone_rms).max(0.0));
        let snr = snr_db(tone_rms, noise_rms);
        let sc = score_0_100(snr, 0.0, 20.0);
        eprintln!("[CW-PEAK] {suffix}: rms={rms_total:.0} tone_snr={snr:.1}dB score={sc}/100");
    }
}

// ============================================================
// Test 9: VOX
// ============================================================
#[test]
fn test_vox() {
    let cordic = make_cordic();
    let speech_i16 = load_wav_i16(&fixtures_dir().join("speech.wav"));
    let speech_u16 = i16_to_u16(&speech_i16);
    let num_blocks = speech_u16.len() / AUDIO_BUFFER_SIZE;
    let zero = [0u16; AUDIO_BUFFER_SIZE];
    let out = output_dir("vox");

    let mut mixer = AudioMixer::new(cordic);
    mixer.set_vox_enabled(true);
    mixer.set_vox_voice_mode(true);
    mixer.set_vox_gain(500);
    mixer.set_vox_delay(300);
    mixer.set_vox_anti_trip(200);

    let mut transitions: Vec<(usize, bool)> = Vec::new();
    let mut tx_output = Vec::with_capacity(speech_u16.len());

    for block in 0..num_blocks {
        let start = block * AUDIO_BUFFER_SIZE;
        let mut mic_buf = [0u16; AUDIO_BUFFER_SIZE];
        mic_buf.copy_from_slice(&speech_u16[start..start + AUDIO_BUFFER_SIZE]);
        mixer.set_buffer_mic(mic_buf);
        mixer.set_buffer_rx(zero);
        mixer.set_buffer_generator(zero);

        if let Some(tx_on) = mixer.process_vox() {
            transitions.push((block, tx_on));
        }

        mixer.mix();
        tx_output.extend_from_slice(&mixer.get_buffer_tx());
    }

    let tx_i16 = u16_to_i16(&tx_output);
    write_wav(&out.join("output-tx.wav"), &tx_i16, 48000);

    let on_count = transitions.iter().filter(|(_, on)| *on).count();
    let off_count = transitions.iter().filter(|(_, on)| !*on).count();
    eprintln!(
        "[VOX] {on_count} on-transitions, {off_count} off-transitions, {} total blocks",
        num_blocks
    );
    for (blk, on) in &transitions {
        let ms = *blk * AUDIO_BUFFER_SIZE * 1000 / 48000;
        eprintln!(
            "[VOX]   block {blk} ({ms}ms): {}",
            if *on { "TX ON" } else { "TX OFF" }
        );
    }
    let sc = score_0_100(on_count as f32, 0.0, 5.0);
    eprintln!("[VOX] score={sc}/100");

    assert!(on_count > 0, "VOX should trigger at least once on speech");
}

// ============================================================
// Test 10: TX EQ (AudioMixer biquad on mic path)
// ============================================================
#[test]
fn test_tx_eq() {
    let cordic = make_cordic();
    let out = output_dir("tx-eq");

    let num_samples = 48000;
    let zero = [0u16; AUDIO_BUFFER_SIZE];

    for &(freq, label) in &[(300.0f32, "300hz"), (1000.0, "1000hz"), (3000.0, "3000hz")] {
        let tone = generate_tone_i16(freq, 48000.0, num_samples, 0.5);
        let tone_u16 = i16_to_u16(&tone);
        let num_blocks = tone_u16.len() / AUDIO_BUFFER_SIZE;

        let process_tx_eq = |cordic_ref, enabled: bool, low: i8, mid: i8, high: i8| -> Vec<u16> {
            let mut mixer = AudioMixer::new(cordic_ref);
            mixer.set_tx_eq_enabled(enabled);
            if enabled {
                mixer.set_tx_eq_low(EqGain::new(low));
                mixer.set_tx_eq_mid(EqGain::new(mid));
                mixer.set_tx_eq_high(EqGain::new(high));
            }
            let mut output = Vec::with_capacity(tone_u16.len());
            for b in 0..num_blocks {
                let start = b * AUDIO_BUFFER_SIZE;
                let mut mic_buf = [0u16; AUDIO_BUFFER_SIZE];
                mic_buf.copy_from_slice(&tone_u16[start..start + AUDIO_BUFFER_SIZE]);
                mixer.set_buffer_mic(mic_buf);
                mixer.set_buffer_rx(zero);
                mixer.set_buffer_generator(zero);
                mixer.mix();
                output.extend_from_slice(&mixer.get_buffer_tx());
            }
            output
        };

        let flat_out = process_tx_eq(cordic, false, 0, 0, 0);
        let flat_i16 = u16_to_i16(&flat_out);

        let (low_b, mid_b, high_b) = if freq < 500.0 {
            (12i8, 0, 0)
        } else if freq <= 2000.0 {
            (0, 12, 0)
        } else {
            (0, 0, 12)
        };
        let boost_out = process_tx_eq(cordic, true, low_b, mid_b, high_b);
        let boost_i16 = u16_to_i16(&boost_out);

        let (low_c, mid_c, high_c) = if freq < 500.0 {
            (-12i8, 0, 0)
        } else if freq <= 2000.0 {
            (0, -12, 0)
        } else {
            (0, 0, -12)
        };
        let cut_out = process_tx_eq(cordic, true, low_c, mid_c, high_c);
        let cut_i16 = u16_to_i16(&cut_out);

        write_wav(&out.join(format!("{label}-flat.wav")), &flat_i16, 48000);
        write_wav(&out.join(format!("{label}-boost.wav")), &boost_i16, 48000);
        write_wav(&out.join(format!("{label}-cut.wav")), &cut_i16, 48000);

        let skip = AUDIO_BUFFER_SIZE * 2;
        let rms_flat = rms_i16(&flat_i16[skip..]);
        let rms_boost = rms_i16(&boost_i16[skip..]);
        let rms_cut = rms_i16(&cut_i16[skip..]);
        let boost_db = 20.0 * libm::log10f(rms_boost / rms_flat.max(1.0));
        let cut_db = 20.0 * libm::log10f(rms_cut / rms_flat.max(1.0));
        eprintln!("[TX-EQ] {label}: flat={rms_flat:.0} boost={rms_boost:.0}(+{boost_db:.1}dB) cut={rms_cut:.0}({cut_db:.1}dB)");

        assert!(
            rms_boost > rms_flat,
            "TX EQ {label}: boost ({rms_boost:.0}) should be louder than flat ({rms_flat:.0})"
        );
        assert!(
            rms_cut < rms_flat,
            "TX EQ {label}: cut ({rms_cut:.0}) should be quieter than flat ({rms_flat:.0})"
        );
    }
}

// ============================================================
// Test 11: Selectivity Filter (pipeline: TX→RX with BPF)
// ============================================================
#[test]
fn test_selectivity_filter() {
    let cordic = make_cordic();
    let out = output_dir("selectivity");

    let dirty_i16 = load_wav_i16(&fixtures_dir().join("speech-with-4khz.wav"));
    let dirty_u16 = i16_to_u16(&dirty_i16);
    let dirty_blocks = to_blocks_u16(&dirty_u16);

    let clean_i16 = load_wav_i16(&fixtures_dir().join("speech.wav"));
    let clean_u16 = i16_to_u16(&clean_i16);
    let clean_blocks = to_blocks_u16(&clean_u16);

    let if_dirty = tx_modulate(cordic, DemodMode::Usb, &dirty_blocks);
    let if_clean = tx_modulate(cordic, DemodMode::Usb, &clean_blocks);

    let raw_clean = rx_process(cordic, DemodMode::Usb, &if_clean, |dsp| {
        dsp.set_filter_enabled(true);
        dsp.set_bandwidth(2700.0);
        dsp.set_shift(0.0);
    });
    let clean_output_i16 = normalize_to_i16(&raw_clean, SKIP);
    write_wav(&out.join("output-clean-ref.wav"), &clean_output_i16, 48000);

    for filter_on in [false, true] {
        let raw = rx_process(cordic, DemodMode::Usb, &if_dirty, |dsp| {
            dsp.set_filter_enabled(filter_on);
            if filter_on {
                dsp.set_bandwidth(2700.0);
                dsp.set_shift(0.0);
            }
        });
        let output_i16 = normalize_to_i16(&raw, SKIP);

        let suffix = if filter_on { "bpf-on" } else { "bpf-off" };
        write_wav(
            &out.join(format!("output-{suffix}.wav")),
            &output_i16,
            48000,
        );

        let rms_total = rms_i16(&output_i16[SKIP..]);

        let measure_tone = |freq: f64| -> f64 {
            let n = output_i16[SKIP..].len() as f64;
            let mut sum_sin = 0.0f64;
            let mut sum_cos = 0.0f64;
            for (i, &s) in output_i16[SKIP..].iter().enumerate() {
                let t = (i + SKIP) as f64 / 48000.0;
                let sf = s as f64 / 32767.0;
                sum_sin += sf * libm::sin(2.0 * core::f64::consts::PI * freq * t);
                sum_cos += sf * libm::cos(2.0 * core::f64::consts::PI * freq * t);
            }
            libm::sqrt(2.0 * (sum_sin * sum_sin + sum_cos * sum_cos) / (n * n))
        };

        let tone_4k_rms = measure_tone(4000.0);
        let sc = if filter_on {
            score_0_100(
                20.0 * libm::log10f(1.0 / (tone_4k_rms as f32).max(1e-6)),
                0.0,
                40.0,
            )
        } else {
            0
        };
        eprintln!(
            "[SELECTIVITY] {suffix}: rms={rms_total:.0} 4kHz_tone={:.6} score={sc}/100",
            tone_4k_rms
        );
    }
}

// ============================================================
// Test 12: Volume + Speaker Routing
// ============================================================
#[test]
fn test_volume_and_routing() {
    let cordic = make_cordic();
    let speech_i16 = load_wav_i16(&fixtures_dir().join("speech.wav"));
    let out = output_dir("volume");

    let speech_u16 = i16_to_u16(&speech_i16);
    let num_blocks = speech_u16.len() / AUDIO_BUFFER_SIZE;
    let zero = [0u16; AUDIO_BUFFER_SIZE];

    for &(vol_raw, hp_connected, label) in &[
        (1000i16, true, "hp-full"),
        (1000, false, "spk-vol100"),
        (500, false, "spk-vol50"),
        (0, false, "spk-vol0"),
    ] {
        let mut mixer = AudioMixer::new(cordic);
        mixer.set_volume(Volume::new(vol_raw));
        mixer.set_headphones_connected(hp_connected);

        let mut hp_output = Vec::with_capacity(speech_u16.len());
        let mut spk_output = Vec::with_capacity(speech_u16.len());

        for block in 0..num_blocks {
            let start = block * AUDIO_BUFFER_SIZE;
            let mut buf = [0u16; AUDIO_BUFFER_SIZE];
            buf.copy_from_slice(&speech_u16[start..start + AUDIO_BUFFER_SIZE]);
            mixer.set_buffer_rx(buf);
            mixer.set_buffer_mic(zero);
            mixer.set_buffer_generator(zero);
            mixer.mix();
            hp_output.extend_from_slice(&mixer.get_buffer_headphones());
            spk_output.extend_from_slice(&mixer.get_buffer_speakers());
        }

        let active_output = if hp_connected {
            &hp_output
        } else {
            &spk_output
        };
        let inactive_output = if hp_connected {
            &spk_output
        } else {
            &hp_output
        };

        let active_i16 = u16_to_i16(active_output);
        write_wav(&out.join(format!("output-{label}.wav")), &active_i16, 48000);

        let skip = AUDIO_BUFFER_SIZE * 2;
        let rms_active = rms_i16(&active_i16[skip..]);
        let inactive_max: u16 = inactive_output.iter().copied().max().unwrap_or(0);
        eprintln!("[VOLUME] {label}: rms={rms_active:.0} inactive_max={inactive_max}");

        assert_eq!(inactive_max, 0, "{label}: inactive output should be silent");

        if !hp_connected && vol_raw == 0 {
            let active_max: u16 = active_output.iter().copied().max().unwrap_or(0);
            assert_eq!(
                active_max, 0,
                "{label}: speaker volume=0 should output zero (max_u16={active_max})"
            );
        }
    }

    let ac_rms_u16 = |data: &[u16], skip: usize| -> f32 {
        let slice = &data[skip..];
        let mean: f64 = slice.iter().map(|&s| s as f64).sum::<f64>() / slice.len() as f64;
        let var: f64 = slice
            .iter()
            .map(|&s| {
                let d = s as f64 - mean;
                d * d
            })
            .sum::<f64>()
            / slice.len() as f64;
        libm::sqrt(var) as f32
    };

    let spk_data = |vol: i16| -> Vec<u16> {
        let mut mixer = AudioMixer::new(cordic);
        mixer.set_volume(Volume::new(vol));
        mixer.set_headphones_connected(false);
        let mut out_spk = Vec::with_capacity(speech_u16.len());
        for block in 0..num_blocks {
            let start = block * AUDIO_BUFFER_SIZE;
            let mut buf = [0u16; AUDIO_BUFFER_SIZE];
            buf.copy_from_slice(&speech_u16[start..start + AUDIO_BUFFER_SIZE]);
            mixer.set_buffer_rx(buf);
            mixer.set_buffer_mic(zero);
            mixer.set_buffer_generator(zero);
            mixer.mix();
            out_spk.extend_from_slice(&mixer.get_buffer_speakers());
        }
        out_spk
    };

    let skip = AUDIO_BUFFER_SIZE * 2;
    let data100 = spk_data(1000);
    let data50 = spk_data(500);
    let rms100 = ac_rms_u16(&data100, skip);
    let rms50 = ac_rms_u16(&data50, skip);
    let vol_diff_db = 20.0 * libm::log10f(rms100 / rms50.max(1.0));
    eprintln!(
        "[VOLUME] scaling (AC-coupled): spk100={rms100:.0} spk50={rms50:.0} diff={vol_diff_db:.1}dB"
    );
    assert!(
        rms100 > rms50,
        "Speaker vol=100% ({rms100:.0}) should be louder than vol=50% ({rms50:.0})"
    );
}
