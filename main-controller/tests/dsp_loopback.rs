use std::cell::RefCell;
use std::path::PathBuf;

use embassy_sync::blocking_mutex::Mutex;

use druzhba_main_controller::consts::{ADC_BUFFER_SIZE, ADC_SAMPLE_RATE, AUDIO_BUFFER_SIZE};
use druzhba_main_controller::cordic_math::{CordicMath, CordicMutex};
use druzhba_main_controller::dsp::types::{AgcPreset, DemodMode, DSP_BLOCK_SIZE};
use druzhba_main_controller::dsp::DspPipeline;
use druzhba_main_controller::tone_generator::ToneGenerator;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test-fixtures")
}

fn output_dir(subdir: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test-output")
        .join(subdir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn make_cordic() -> &'static CordicMutex {
    let m: &'static CordicMutex = Box::leak(Box::new(Mutex::new(RefCell::new(CordicMath::new()))));
    m
}

fn load_wav_u16(path: &std::path::Path) -> Vec<u16> {
    let mut reader = hound::WavReader::open(path).unwrap();
    reader
        .samples::<i16>()
        .map(|s| (s.unwrap() as i32 + 32768) as u16)
        .collect()
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

fn to_blocks(samples: &[u16]) -> Vec<[u16; AUDIO_BUFFER_SIZE]> {
    let num_blocks = samples.len() / AUDIO_BUFFER_SIZE;
    let mut blocks = Vec::with_capacity(num_blocks);
    for block in 0..num_blocks {
        let start = block * AUDIO_BUFFER_SIZE;
        let mut buf = [0u16; AUDIO_BUFFER_SIZE];
        buf.copy_from_slice(&samples[start..start + AUDIO_BUFFER_SIZE]);
        blocks.push(buf);
    }
    blocks
}

fn mode_name(mode: DemodMode) -> &'static str {
    match mode {
        DemodMode::Usb => "usb",
        DemodMode::Lsb => "lsb",
        DemodMode::Cw => "cw",
        DemodMode::Am => "am",
        DemodMode::Fm => "fm",
        DemodMode::Sam => "sam",
    }
}

fn tx_then_rx(
    cordic: &'static CordicMutex,
    mode: DemodMode,
    audio_blocks: &[[u16; AUDIO_BUFFER_SIZE]],
    cw_key_down: bool,
) -> (Vec<i16>, Vec<i16>) {
    let mut dsp_tx = DspPipeline::new(cordic);
    dsp_tx.set_demod_mode(mode);
    dsp_tx.set_filter_enabled(true);
    dsp_tx.set_compressor_enabled(false);
    if cw_key_down {
        dsp_tx.set_cw_key(true);
    }

    let mut tx_if_buffers: Vec<[u32; ADC_BUFFER_SIZE]> = Vec::with_capacity(audio_blocks.len());

    for audio_block in audio_blocks {
        let mut dac_out = [0u32; ADC_BUFFER_SIZE];
        dsp_tx.process_tx(audio_block, &mut dac_out);
        tx_if_buffers.push(dac_out);
    }

    let if_samples: Vec<i16> = tx_if_buffers
        .iter()
        .flat_map(|buf| {
            buf.iter().step_by(2).map(|&s| {
                let signed_24 = ((s << 8) as i32) >> 8;
                (signed_24 >> 8) as i16
            })
        })
        .collect();

    let mut dsp_rx = DspPipeline::new(cordic);
    dsp_rx.set_demod_mode(mode);
    dsp_rx.set_filter_enabled(true);
    dsp_rx.set_agc_preset(AgcPreset::Off);

    let mut raw_samples: Vec<f32> = Vec::with_capacity(audio_blocks.len() * DSP_BLOCK_SIZE);
    for if_buf in &tx_if_buffers {
        let block = dsp_rx.process_rx_raw(if_buf);
        raw_samples.extend_from_slice(&block);
    }

    let skip_samples = AUDIO_BUFFER_SIZE * 4;
    let global_peak = raw_samples[skip_samples..]
        .iter()
        .fold(0.0f32, |mx, &s| mx.max(s.abs()));
    let scale = if global_peak > 1e-6 {
        1.0 / global_peak
    } else {
        1.0
    };

    let output_samples: Vec<i16> = raw_samples
        .iter()
        .map(|&s| {
            let normalized = s * scale;
            (normalized * 32767.0).clamp(-32768.0, 32767.0) as i16
        })
        .collect();

    (if_samples, output_samples)
}

fn peak_amplitude(samples: &[i16]) -> i16 {
    let mut peak: i16 = 0;
    for &s in samples {
        let abs = s.saturating_abs();
        if abs > peak {
            peak = abs;
        }
    }
    peak
}

const VOICE_MODES: &[DemodMode] = &[
    DemodMode::Usb,
    DemodMode::Lsb,
    DemodMode::Am,
    DemodMode::Fm,
    DemodMode::Sam,
];

fn find_lag(a: &[f32], b: &[f32], max_lag: usize) -> (i32, f32) {
    let mut best_corr: f32 = -2.0;
    let mut best_lag: i32 = 0;
    let n = a.len().min(b.len());
    for lag in -(max_lag as i32)..=(max_lag as i32) {
        let mut sum = 0.0f32;
        let mut count = 0u32;
        for i in 0..n {
            let j = i as i32 + lag;
            if j >= 0 && (j as usize) < n {
                sum += a[i] * b[j as usize];
                count += 1;
            }
        }
        if count > 0 {
            let corr = sum / count as f32;
            if corr > best_corr {
                best_corr = corr;
                best_lag = lag;
            }
        }
    }
    (best_lag, best_corr)
}

fn rms_error_at_lag(a: &[f32], b: &[f32], lag: i32) -> f32 {
    let n = a.len().min(b.len());
    let mut sum_sq = 0.0f32;
    let mut count = 0u32;
    for i in 0..n {
        let j = i as i32 + lag;
        if j >= 0 && (j as usize) < n {
            let diff = a[i] - b[j as usize];
            sum_sq += diff * diff;
            count += 1;
        }
    }
    libm::sqrtf(sum_sq / count as f32)
}

fn normalize(samples: &[f32]) -> Vec<f32> {
    let peak = samples.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
    if peak < 1.0 {
        return samples.to_vec();
    }
    samples.iter().map(|x| x / peak).collect()
}

fn run_voice_loopback(
    cordic: &'static CordicMutex,
    prefix: &str,
    mode: DemodMode,
    audio_blocks: &[[u16; AUDIO_BUFFER_SIZE]],
    max_rms: f32,
) {
    let (if_samples, rx_samples) = tx_then_rx(cordic, mode, audio_blocks, false);
    let name = mode_name(mode);

    let out = output_dir("loopback");

    write_wav(
        &out.join(format!("{prefix}-tx-{name}.wav")),
        &if_samples,
        ADC_SAMPLE_RATE,
    );
    write_wav(
        &out.join(format!("{prefix}-rx-{name}.wav")),
        &rx_samples,
        48000,
    );

    let skip = AUDIO_BUFFER_SIZE * 4;
    let peak = peak_amplitude(&rx_samples[skip..]);
    assert!(
        peak > 1000,
        "{prefix} {name}: signal too weak after loopback (peak={peak}, min=1000)",
    );

    let input_flat: Vec<f32> = audio_blocks
        .iter()
        .flat_map(|buf| buf.iter().map(|&s| s as f32 - 32768.0))
        .collect();
    let rx_flat: Vec<f32> = rx_samples.iter().map(|&s| s as f32).collect();

    let analysis_len = input_flat.len() - skip;
    let in_slice = &input_flat[skip..skip + analysis_len];
    let rx_slice = &rx_flat[skip..skip + analysis_len];

    let in_norm = normalize(in_slice);
    let rx_norm = normalize(rx_slice);

    let (lag, _corr) = find_lag(&in_norm, &rx_norm, 512);
    let rms = rms_error_at_lag(&in_norm, &rx_norm, lag);
    eprintln!("{prefix} {name}: peak={peak} lag={lag} rms={rms:.4}");

    assert!(
        rms < max_rms,
        "{prefix} {name}: fidelity too low (rms={rms:.4}, max={max_rms}, lag={lag})",
    );
}

#[test]
fn test_dsp_loopback_tone_1khz() {
    let cordic = make_cordic();
    let input = load_wav_u16(&fixtures_dir().join("tone-1khz.wav"));
    let blocks = to_blocks(&input);

    for &mode in VOICE_MODES {
        run_voice_loopback(cordic, "tone-1khz", mode, &blocks, 0.06);
    }
}

#[test]
fn test_dsp_loopback_tone_generator() {
    let cordic = make_cordic();

    let mut tone_gen = ToneGenerator::new(cordic);
    tone_gen.set_tone_active(true);

    let num_blocks = 188;
    let mut blocks: Vec<[u16; AUDIO_BUFFER_SIZE]> = Vec::with_capacity(num_blocks);
    for _ in 0..num_blocks {
        blocks.push(tone_gen.next_buffer());
    }

    let input_i16: Vec<i16> = blocks
        .iter()
        .flat_map(|buf| buf.iter().map(|&s| (s as i32 - 32768) as i16))
        .collect();

    let out = output_dir("loopback");
    write_wav(&out.join("tone-generator-input.wav"), &input_i16, 48000);

    for &mode in VOICE_MODES {
        run_voice_loopback(cordic, "tone-generator", mode, &blocks, 0.06);
    }
}

#[test]
fn test_dsp_loopback_speech() {
    let cordic = make_cordic();
    let input = load_wav_u16(&fixtures_dir().join("speech.wav"));
    let blocks = to_blocks(&input);

    for &mode in VOICE_MODES {
        let max_rms = match mode {
            DemodMode::Usb | DemodMode::Lsb => 0.12,
            _ => 0.06,
        };
        run_voice_loopback(cordic, "speech", mode, &blocks, max_rms);
    }
}

#[test]
fn test_dsp_loopback_idempotent() {
    let cordic = make_cordic();
    let input = load_wav_u16(&fixtures_dir().join("speech.wav"));
    let skip = AUDIO_BUFFER_SIZE * 4;

    for &mode in VOICE_MODES {
        let name = mode_name(mode);
        let mut blocks = to_blocks(&input);
        let mut prev_samples: Option<Vec<i16>> = None;

        let out = output_dir("idempotent");

        for pass in 1..=10 {
            let (_, rx) = tx_then_rx(cordic, mode, &blocks, false);

            if pass == 1 || pass == 10 {
                write_wav(&out.join(format!("{name}-pass{pass:02}.wav")), &rx, 48000);
            }

            if let Some(ref prev) = prev_samples {
                let prev_f: Vec<f32> = prev[skip..].iter().map(|&s| s as f32).collect();
                let curr_f: Vec<f32> = rx[skip..].iter().map(|&s| s as f32).collect();
                let prev_n = normalize(&prev_f);
                let curr_n = normalize(&curr_f);
                let (lag, _) = find_lag(&prev_n, &curr_n, 512);
                let rms = rms_error_at_lag(&prev_n, &curr_n, lag);
                eprintln!(
                    "{name} pass {prev}→{pass}: rms={rms:.4} lag={lag}",
                    prev = pass - 1
                );

                assert!(
                    rms < 0.10,
                    "{name}: excessive degradation at pass {prev}→{pass} (rms={rms:.4}, max=0.10)",
                    prev = pass - 1
                );
            }

            let rx_u16: Vec<u16> = rx.iter().map(|&s| (s as i32 + 32768) as u16).collect();
            blocks = to_blocks(&rx_u16);
            prev_samples = Some(rx);
        }
    }
}

#[test]
fn test_dsp_loopback_cw() {
    let cordic = make_cordic();

    let silence = [32768u16; AUDIO_BUFFER_SIZE];
    let num_blocks = 188;
    let blocks: Vec<[u16; AUDIO_BUFFER_SIZE]> = vec![silence; num_blocks];

    let (if_samples, rx_samples) = tx_then_rx(cordic, DemodMode::Cw, &blocks, true);

    let out = output_dir("cw");

    write_wav(&out.join("cw-tx.wav"), &if_samples, ADC_SAMPLE_RATE);
    write_wav(&out.join("cw-rx.wav"), &rx_samples, 48000);

    let skip = AUDIO_BUFFER_SIZE * 4;
    let peak = peak_amplitude(&rx_samples[skip..]);
    eprintln!("cw loopback peak: {peak}");
    assert!(
        peak > 1000,
        "cw: signal too weak after loopback (peak={peak}, min=1000)",
    );
}
