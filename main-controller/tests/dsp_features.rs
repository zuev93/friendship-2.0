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

const SKIP: usize = AUDIO_BUFFER_SIZE * 4;

fn add_impulse_noise_u32(
    if_buffers: &[[u32; ADC_BUFFER_SIZE]],
    seed: u64,
    probability: u32,
    amplitude_24: i32,
) -> Vec<[u32; ADC_BUFFER_SIZE]> {
    let mut rng = seed;
    let mut out = if_buffers.to_vec();
    let stereo_frames = ADC_BUFFER_SIZE / 2;
    for buf in &mut out {
        for frame in 0..stereo_frames {
            rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
            if (rng >> 32) as u32 % probability == 0 {
                let noise_sign = if (rng >> 16) & 1 == 0 { 1i32 } else { -1 };
                let idx = frame * 2;
                let signed_24 = ((buf[idx] << 8) as i32) >> 8;
                let noisy = (signed_24 + noise_sign * amplitude_24).clamp(-8_388_607, 8_388_607);
                buf[idx] = (noisy & 0x00FF_FFFF) as u32;
            }
        }
    }
    out
}

// ============================================================
// Test 1: Noise Blanker (IF-level impulse noise → RX)
// ============================================================
#[test]
fn test_noise_blanker() {
    let cordic = make_cordic();
    let clean_i16 = load_wav_i16(&fixtures_dir().join("speech.wav"));
    let clean_u16 = i16_to_u16(&clean_i16);
    let clean_blocks = to_blocks_u16(&clean_u16);
    let out = output_dir("nb");

    let if_clean = tx_modulate(cordic, DemodMode::Usb, &clean_blocks);

    let if_noisy = add_impulse_noise_u32(&if_clean, 12345, 500, 4_000_000);

    let raw_clean_ref = rx_process(cordic, DemodMode::Usb, &if_clean, |_| {});

    let nb_snr = |raw: &[f32], ref_raw: &[f32]| -> f32 {
        let rms_diff = rms_f32(
            &raw[SKIP..]
                .iter()
                .zip(ref_raw[SKIP..].iter())
                .map(|(&a, &b)| a - b)
                .collect::<Vec<_>>(),
        );
        let rms_clean = rms_f32(&ref_raw[SKIP..]);
        snr_db(rms_clean, rms_diff)
    };

    let raw_off = rx_process(cordic, DemodMode::Usb, &if_noisy, |_| {});
    write_wav(
        &out.join("output-nb-off.wav"),
        &normalize_to_i16(&raw_off, SKIP),
        48000,
    );
    let snr_off = nb_snr(&raw_off, &raw_clean_ref);

    let raw_on = rx_process(cordic, DemodMode::Usb, &if_noisy, |dsp| {
        dsp.set_nb_enabled(true);
        dsp.set_nb_threshold(4);
    });
    write_wav(
        &out.join("output-nb-on.wav"),
        &normalize_to_i16(&raw_on, SKIP),
        48000,
    );
    let snr_on = nb_snr(&raw_on, &raw_clean_ref);

    let improvement = snr_on - snr_off;
    eprintln!("[NB] improvement={improvement:.1}dB (off={snr_off:.1}dB on={snr_on:.1}dB)");
    assert!(
        improvement > 1.5,
        "NB must improve SNR by at least 1.5dB (got {improvement:.1}dB)",
    );
    assert!(
        snr_on > snr_off,
        "NB on ({snr_on:.1}dB) must be better than NB off ({snr_off:.1}dB)",
    );
}

// ============================================================
// Test 2: Noise Reduction (Spectral Subtraction in AudioMixer)
// ============================================================
#[test]
fn test_noise_reduction() {
    let cordic = make_cordic();
    let clean_i16 = load_wav_i16(&fixtures_dir().join("speech.wav"));
    let noisy_i16 = load_wav_i16(&fixtures_dir().join("speech-white-noise.wav"));
    let noisy_u16 = i16_to_u16(&noisy_i16);
    let out = output_dir("nr");

    let clean_u16 = i16_to_u16(&clean_i16);
    let mut mixer_clean = AudioMixer::new(cordic);
    let clean_ref_output = mixer_process_blocks(&mut mixer_clean, &clean_u16);
    let clean_ref_i16 = u16_to_i16(&clean_ref_output);

    let nr_snr = |output_i16: &[i16], ref_i16: &[i16]| -> f32 {
        let len = output_i16.len().min(ref_i16.len());
        let noise_est: Vec<f32> = output_i16[SKIP..len]
            .iter()
            .zip(ref_i16[SKIP..len].iter())
            .map(|(&a, &b)| (a - b) as f32)
            .collect();
        let noise_rms = rms_f32(&noise_est);
        let clean_rms = rms_i16(&ref_i16[SKIP..len]);
        snr_db(clean_rms, noise_rms)
    };

    let mut mixer_off = AudioMixer::new(cordic);
    let off_output = mixer_process_blocks(&mut mixer_off, &noisy_u16);
    let off_i16 = u16_to_i16(&off_output);
    write_wav(&out.join("output-nr-off.wav"), &off_i16, 48000);
    let snr_off = nr_snr(&off_i16, &clean_ref_i16);

    let mut mixer_on = AudioMixer::new(cordic);
    mixer_on.set_nr_enabled(true);
    mixer_on.set_nr_level(NrLevel::new(500));
    let on_output = mixer_process_blocks(&mut mixer_on, &noisy_u16);
    let on_i16 = u16_to_i16(&on_output);
    write_wav(&out.join("output-nr-on.wav"), &on_i16, 48000);
    let snr_on = nr_snr(&on_i16, &clean_ref_i16);

    let improvement = snr_on - snr_off;
    eprintln!("[NR] improvement={improvement:.1}dB (off={snr_off:.1}dB on={snr_on:.1}dB)");
    assert!(
        improvement > 3.0,
        "NR must improve SNR by at least 3dB (got {improvement:.1}dB)",
    );
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

    let skip_anf = AUDIO_BUFFER_SIZE * 16;
    let anf_reduction = |output_i16: &[i16]| -> f32 {
        let len = clean_i16.len().min(output_i16.len());
        let residual_tone: Vec<f32> = clean_i16[skip_anf..len]
            .iter()
            .zip(output_i16[skip_anf..len].iter())
            .map(|(&a, &b)| (b - a) as f32)
            .collect();
        let tone_rms_out = rms_f32(&residual_tone);
        20.0 * libm::log10f(tone_rms_in / tone_rms_out.max(1.0))
    };

    let mut mixer_off = AudioMixer::new(cordic);
    let off_output = mixer_process_blocks(&mut mixer_off, &noisy_u16);
    let off_i16 = u16_to_i16(&off_output);
    write_wav(&out.join("output-anf-off.wav"), &off_i16, 48000);
    let db_off = anf_reduction(&off_i16);

    let mut mixer_on = AudioMixer::new(cordic);
    mixer_on.set_anf_enabled(true);
    let on_output = mixer_process_blocks(&mut mixer_on, &noisy_u16);
    let on_i16 = u16_to_i16(&on_output);
    write_wav(&out.join("output-anf-on.wav"), &on_i16, 48000);
    let db_on = anf_reduction(&on_i16);

    eprintln!("[ANF] reduction={db_on:.1}dB (off={db_off:.1}dB)");
    assert!(
        db_on > 6.0,
        "ANF must reduce tone by at least 6dB (got {db_on:.1}dB)",
    );
    assert!(
        db_on > db_off + 3.0,
        "ANF on ({db_on:.1}dB) must beat ANF off ({db_off:.1}dB) by at least 3dB",
    );
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
    eprintln!("[SQUELCH] {silent_blocks}/{total_blocks} silenced ({pct}%, expected ~33%)");
    assert!(
        pct >= 25 && pct <= 45,
        "squelch should silence ~33% of blocks (got {pct}%)",
    );
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

    let spread_db = |raw: &[f32]| -> f32 {
        let third = raw.len() / 3;
        let rms1 = rms_f32(&raw[SKIP..third]);
        let rms2 = rms_f32(&raw[third..2 * third]);
        let rms3 = rms_f32(&raw[2 * third..]);
        let max_rms = rms1.max(rms2).max(rms3);
        let min_rms = rms1.min(rms2).min(rms3);
        20.0 * libm::log10f(max_rms / min_rms.max(1e-10))
    };

    let raw_off = rx_process_with_agc(cordic, DemodMode::Usb, &if_buffers, AgcPreset::Off);
    let off_i16 = normalize_to_i16(&raw_off, SKIP);
    write_wav(&out.join("output-agc-off.wav"), &off_i16, 48000);
    let spread_off = spread_db(&raw_off);

    let mut best_spread = f32::MAX;
    let mut best_name = "";
    for &(preset, name) in &[(AgcPreset::SsbFast, "fast"), (AgcPreset::SsbSlow, "slow")] {
        let raw = rx_process_with_agc(cordic, DemodMode::Usb, &if_buffers, preset);
        let output_i16 = normalize_to_i16(&raw, SKIP);
        write_wav(
            &out.join(format!("output-agc-{name}.wav")),
            &output_i16,
            48000,
        );
        let s = spread_db(&raw);
        eprintln!("[AGC] {name}: spread={s:.1}dB");
        if s < best_spread {
            best_spread = s;
            best_name = name;
        }
    }

    let reduction = spread_off - best_spread;
    eprintln!(
        "[AGC] off spread={spread_off:.1}dB, best={best_name} spread={best_spread:.1}dB, reduction={reduction:.1}dB"
    );
    assert!(
        best_spread < spread_off,
        "AGC must reduce level spread (off={spread_off:.1}dB, best={best_spread:.1}dB)",
    );
    assert!(
        reduction > 3.0,
        "AGC must reduce spread by at least 3dB (got {reduction:.1}dB)",
    );
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
        let cut_db = 20.0 * libm::log10f(rms_flat / rms_cut.max(1.0));

        eprintln!("[EQ] {label}: boost={boost_db:.1}dB cut={cut_db:.1}dB");

        assert!(
            boost_db > 3.0,
            "EQ {label}: boost must be at least 3dB (got {boost_db:.1}dB)"
        );
        assert!(
            cut_db > 3.0,
            "EQ {label}: cut must attenuate at least 3dB (got {cut_db:.1}dB)"
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

    let mut results: Vec<(i16, f32)> = Vec::new();

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
        results.push((comp_level, rms));
    }

    let (_, rms0) = results[0];
    let (_, rms1000) = results[2];
    let compression_db = 20.0 * libm::log10f(rms0 / rms1000.max(1.0));
    eprintln!("[COMPRESSOR] {compression_db:.1}dB dynamic range reduction at level=1000");
    assert!(
        compression_db > 3.0,
        "compressor at level=1000 must reduce dynamic range by at least 3dB (got {compression_db:.1}dB)",
    );
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

    let cw_peak_snr = |output_i16: &[i16]| -> f32 {
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
        snr_db(tone_rms, noise_rms)
    };

    let raw_off = rx_process(cordic, DemodMode::Usb, &if_buffers, |_| {});
    let off_i16 = normalize_to_i16(&raw_off, SKIP);
    write_wav(&out.join("output-peak-off.wav"), &off_i16, 48000);
    let snr_off = cw_peak_snr(&off_i16);

    let raw_on = rx_process(cordic, DemodMode::Usb, &if_buffers, |dsp| {
        dsp.set_cw_peak_enabled(true);
        dsp.set_cw_pitch(700);
        dsp.set_cw_peak_bw(200.0);
    });
    let on_i16 = normalize_to_i16(&raw_on, SKIP);
    write_wav(&out.join("output-peak-on.wav"), &on_i16, 48000);
    let snr_on = cw_peak_snr(&on_i16);

    let improvement = snr_on - snr_off;
    eprintln!("[CW-PEAK] improvement={improvement:.1}dB (off={snr_off:.1}dB on={snr_on:.1}dB)");
    assert!(
        improvement > 1.5,
        "CW peak filter must improve 700Hz SNR by at least 1.5dB (got {improvement:.1}dB)",
    );
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
    mixer.set_vox_gain(50);
    mixer.set_vox_delay(300);
    mixer.set_vox_anti_trip(0);

    let mut tx_output = Vec::with_capacity(speech_u16.len());
    let mut vox_active = false;
    let mut active_blocks = 0u32;

    for block in 0..num_blocks {
        let start = block * AUDIO_BUFFER_SIZE;
        let mut mic_buf = [0u16; AUDIO_BUFFER_SIZE];
        mic_buf.copy_from_slice(&speech_u16[start..start + AUDIO_BUFFER_SIZE]);
        mixer.set_buffer_mic(mic_buf);
        mixer.set_buffer_rx(zero);
        mixer.set_buffer_generator(zero);

        if let Some(tx_on) = mixer.process_vox() {
            vox_active = tx_on;
        }
        if vox_active {
            active_blocks += 1;
        }

        mixer.mix();
        tx_output.extend_from_slice(&mixer.get_buffer_tx());
    }

    let tx_i16 = u16_to_i16(&tx_output);
    write_wav(&out.join("output-tx.wav"), &tx_i16, 48000);

    let active_pct = active_blocks as f32 * 100.0 / num_blocks as f32;
    eprintln!("[VOX] {active_blocks}/{num_blocks} blocks active ({active_pct:.0}%)");
    assert!(
        active_pct > 20.0,
        "VOX must activate on at least 20% of speech blocks (got {active_pct:.0}%)",
    );
    assert!(
        active_pct < 100.0,
        "VOX must not be stuck on 100% (got {active_pct:.0}%)",
    );
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
        let cut_db = 20.0 * libm::log10f(rms_flat / rms_cut.max(1.0));

        eprintln!("[TX-EQ] {label}: boost={boost_db:.1}dB cut={cut_db:.1}dB");

        assert!(
            boost_db > 3.0,
            "TX EQ {label}: boost must be at least 3dB (got {boost_db:.1}dB)"
        );
        assert!(
            cut_db > 3.0,
            "TX EQ {label}: cut must attenuate at least 3dB (got {cut_db:.1}dB)"
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

    let measure_tone_4k = |output_i16: &[i16]| -> f32 {
        let n = output_i16[SKIP..].len() as f64;
        let mut sum_sin = 0.0f64;
        let mut sum_cos = 0.0f64;
        for (i, &s) in output_i16[SKIP..].iter().enumerate() {
            let t = (i + SKIP) as f64 / 48000.0;
            let sf = s as f64 / 32767.0;
            sum_sin += sf * libm::sin(2.0 * core::f64::consts::PI * 4000.0 * t);
            sum_cos += sf * libm::cos(2.0 * core::f64::consts::PI * 4000.0 * t);
        }
        let tone_rms = libm::sqrt(2.0 * (sum_sin * sum_sin + sum_cos * sum_cos) / (n * n));
        20.0 * libm::log10f(1.0 / (tone_rms as f32).max(1e-6))
    };

    let raw_off = rx_process(cordic, DemodMode::Usb, &if_dirty, |dsp| {
        dsp.set_filter_enabled(false);
    });
    let off_i16 = normalize_to_i16(&raw_off, SKIP);
    write_wav(&out.join("output-bpf-off.wav"), &off_i16, 48000);
    let rejection_off = measure_tone_4k(&off_i16);

    let raw_on = rx_process(cordic, DemodMode::Usb, &if_dirty, |dsp| {
        dsp.set_filter_enabled(true);
        dsp.set_bandwidth(2700.0);
        dsp.set_shift(0.0);
    });
    let on_i16 = normalize_to_i16(&raw_on, SKIP);
    write_wav(&out.join("output-bpf-on.wav"), &on_i16, 48000);
    let rejection_on = measure_tone_4k(&on_i16);

    eprintln!("[SELECTIVITY] off={rejection_off:.0}dB on={rejection_on:.0}dB");
    assert!(
        rejection_on > 20.0,
        "BPF must reject 4kHz by at least 20dB (got {rejection_on:.0}dB)",
    );
    assert!(
        rejection_on > rejection_off + 10.0,
        "BPF on ({rejection_on:.0}dB) must beat off ({rejection_off:.0}dB) by at least 10dB",
    );
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

        let inactive_max: u16 = inactive_output.iter().copied().max().unwrap_or(0);

        assert_eq!(inactive_max, 0, "{label}: inactive output should be silent");

        if !hp_connected && vol_raw == 0 {
            let active_max: u16 = active_output.iter().copied().max().unwrap_or(0);
            assert_eq!(
                active_max, 32768,
                "{label}: speaker volume=0 should output silence (max_u16={active_max})"
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
    eprintln!("[VOLUME] 100% vs 50% diff={vol_diff_db:.1}dB");
    assert!(
        vol_diff_db > 3.0,
        "volume 100% must be at least 3dB louder than 50% (got {vol_diff_db:.1}dB)",
    );
}
