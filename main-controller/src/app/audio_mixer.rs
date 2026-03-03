use embassy_stm32::peripherals::{CORDIC, FMAC};
use embassy_stm32::Peri;

use crate::{
    app::{
        cordic_math::CordicMath,
        fmac_fir::{FmacFir, FIR_TAPS},
        types::{AudioAgcMode, Compression, EqGain, NrLevel, Volume},
        vox::VoxProcessor,
    },
    consts::AUDIO_BUFFER_SIZE,
};

const NR_TAPS: usize = 64;
const NR_DELAY: usize = 1;
const ANF_TAPS: usize = 48;
const ANF_DELAY: usize = 8;
const SAMPLE_RATE: f32 = 48000.0;

const MAX_VOLUME_RAW: i16 = 1000;

#[derive(Copy, Clone)]
struct Gains {
    rx_to_hp: u8,
    gen_to_hp: u8,
    rx_to_spk: u8,
    gen_to_spk: u8,
    mic_to_tx: u8,
    gen_to_tx: u8,
}

const GAINS: Gains = Gains {
    rx_to_hp: 255,
    gen_to_hp: 255,
    rx_to_spk: 255,
    gen_to_spk: 255,
    mic_to_tx: 255,
    gen_to_tx: 255,
};

const ATTACK_COEFF: u32 = 128;
const RELEASE_COEFF: u32 = 8;
const NOISE_FLOOR: u32 = 400;

#[derive(Copy, Clone)]
struct BiquadState {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl BiquadState {
    const fn zero() -> Self {
        Self {
            b0: 0.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }

    fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }
}

fn lms_filter(
    buffer: &mut [f32],
    weights: &mut [f32],
    history: &mut [f32],
    hist_idx: &mut usize,
    mu: f32,
    taps: usize,
    delay: usize,
) {
    let hist_len = taps + delay;

    for sample in buffer.iter_mut() {
        let x = *sample;

        history[*hist_idx] = x;
        *hist_idx = (*hist_idx + 1) % hist_len;

        let mut y: f32 = 0.0;
        for k in 0..taps {
            let delay_idx = (*hist_idx + hist_len - delay - 1 - k) % hist_len;
            y += weights[k] * history[delay_idx];
        }

        let error = x - y;

        for k in 0..taps {
            let delay_idx = (*hist_idx + hist_len - delay - 1 - k) % hist_len;
            weights[k] += mu * error * history[delay_idx];
        }

        *sample = error;
    }
}

pub struct AudioMixer {
    rx: [u16; AUDIO_BUFFER_SIZE],
    generator: [u16; AUDIO_BUFFER_SIZE],
    mic: [u16; AUDIO_BUFFER_SIZE],
    out_headphones: [u16; AUDIO_BUFFER_SIZE],
    out_tx: [u16; AUDIO_BUFFER_SIZE],
    out_speakers: [u16; AUDIO_BUFFER_SIZE],
    volume_gain: u8,
    headphones_connected: bool,
    squelch_open: bool,
    squelch_threshold_dbm: i8,
    gains: Gains,
    compression_level: i16,
    envelope: u32,
    gain_reduction: u8,
    nr_enabled: bool,
    nr_mu: u16,
    nr_weights: [f32; NR_TAPS],
    nr_history: [f32; NR_TAPS + NR_DELAY],
    nr_hist_idx: usize,
    usb_tx: [u16; AUDIO_BUFFER_SIZE],
    usb_tx_active: bool,
    usb_tx_timeout: u16,
    dsp_filter_enabled: bool,
    fir_coeffs: [f32; FIR_TAPS],
    bw_hz: u16,
    pbt_hz: i16,
    cw_peak_enabled: bool,
    cw_peak_bq: BiquadState,
    anf_enabled: bool,
    anf_weights: [f32; ANF_TAPS],
    anf_history: [f32; ANF_TAPS + ANF_DELAY],
    anf_hist_idx: usize,
    audio_agc_mode: AudioAgcMode,
    agc_envelope: f32,
    agc_gain: f32,
    tx_eq_enabled: bool,
    tx_eq_low_db: i8,
    tx_eq_mid_db: i8,
    tx_eq_high_db: i8,
    tx_eq_biquads: [BiquadState; 3],
    rx_eq_enabled: bool,
    rx_eq_low_db: i8,
    rx_eq_mid_db: i8,
    rx_eq_high_db: i8,
    rx_eq_biquads: [BiquadState; 3],
    cw_pitch_hz: u16,
    sam_enabled: bool,
    sam_pll_phase: f32,
    sam_pll_freq: f32,
    cordic: CordicMath,
    fmac: FmacFir,
    vox: VoxProcessor,
}

impl AudioMixer {
    pub fn new(cordic_peri: Peri<'static, CORDIC>, fmac_peri: Peri<'static, FMAC>) -> Self {
        Self {
            rx: [0; AUDIO_BUFFER_SIZE],
            generator: [0; AUDIO_BUFFER_SIZE],
            mic: [0; AUDIO_BUFFER_SIZE],
            out_headphones: [0; AUDIO_BUFFER_SIZE],
            out_tx: [0; AUDIO_BUFFER_SIZE],
            out_speakers: [0; AUDIO_BUFFER_SIZE],
            volume_gain: 255,
            headphones_connected: true,
            squelch_open: true,
            squelch_threshold_dbm: -120,
            gains: GAINS,
            compression_level: 0,
            envelope: 0,
            gain_reduction: 0,
            nr_enabled: false,
            nr_mu: 0,
            nr_weights: [0.0; NR_TAPS],
            nr_history: [0.0; NR_TAPS + NR_DELAY],
            nr_hist_idx: 0,
            usb_tx: [32768; AUDIO_BUFFER_SIZE],
            usb_tx_active: false,
            usb_tx_timeout: 0,
            dsp_filter_enabled: false,
            fir_coeffs: [0.0; FIR_TAPS],
            bw_hz: 2700,
            pbt_hz: 0,
            cw_peak_enabled: false,
            cw_peak_bq: BiquadState::zero(),
            anf_enabled: false,
            anf_weights: [0.0; ANF_TAPS],
            anf_history: [0.0; ANF_TAPS + ANF_DELAY],
            anf_hist_idx: 0,
            audio_agc_mode: AudioAgcMode::Off,
            agc_envelope: 0.0,
            agc_gain: 1.0,
            tx_eq_enabled: false,
            tx_eq_low_db: 0,
            tx_eq_mid_db: 0,
            tx_eq_high_db: 0,
            tx_eq_biquads: [BiquadState::zero(); 3],
            rx_eq_enabled: false,
            rx_eq_low_db: 0,
            rx_eq_mid_db: 0,
            rx_eq_high_db: 0,
            rx_eq_biquads: [BiquadState::zero(); 3],
            cw_pitch_hz: 700,
            sam_enabled: false,
            sam_pll_phase: 0.0,
            sam_pll_freq: 0.0,
            cordic: CordicMath::new(cordic_peri),
            fmac: FmacFir::new(fmac_peri),
            vox: VoxProcessor::new(),
        }
    }

    pub fn get_buffer_headphones(&self) -> [u16; AUDIO_BUFFER_SIZE] {
        self.out_headphones
    }

    pub fn get_buffer_tx(&self) -> [u16; AUDIO_BUFFER_SIZE] {
        self.out_tx
    }

    pub fn get_buffer_speakers(&self) -> [u16; AUDIO_BUFFER_SIZE] {
        self.out_speakers
    }

    pub fn set_buffer_rx(&mut self, buffer: [u16; AUDIO_BUFFER_SIZE]) {
        self.rx = buffer;
    }

    pub fn set_buffer_generator(&mut self, buffer: [u16; AUDIO_BUFFER_SIZE]) {
        self.generator = buffer;
    }

    pub fn set_buffer_mic(&mut self, buffer: [u16; AUDIO_BUFFER_SIZE]) {
        self.mic = buffer;
    }

    pub fn set_volume(&mut self, volume: Volume) {
        let raw = volume.raw().max(0) as u32;
        self.volume_gain = ((raw * 255) / MAX_VOLUME_RAW as u32) as u8;
    }

    pub fn set_headphones_connected(&mut self, connected: bool) {
        self.headphones_connected = connected;
    }

    pub fn set_squelch_threshold(&mut self, threshold_dbm: i8) {
        self.squelch_threshold_dbm = threshold_dbm;
    }

    pub fn update_squelch(&mut self, rssi_dbm: i8) {
        self.squelch_open = rssi_dbm >= self.squelch_threshold_dbm;
    }

    pub fn set_compression(&mut self, compression: Compression) {
        self.compression_level = compression.raw();
    }

    pub fn set_nr_enabled(&mut self, enabled: bool) {
        self.nr_enabled = enabled;
        if !enabled {
            self.nr_weights = [0.0; NR_TAPS];
            self.nr_history = [0.0; NR_TAPS + NR_DELAY];
            self.nr_hist_idx = 0;
        }
    }

    pub fn set_buffer_usb_tx(&mut self, buffer: [u16; AUDIO_BUFFER_SIZE]) {
        self.usb_tx = buffer;
        self.usb_tx_active = true;
        self.usb_tx_timeout = 0;
    }

    pub fn set_nr_level(&mut self, level: NrLevel) {
        self.nr_mu = level.raw() as u16;
    }

    pub fn gain_reduction(&self) -> u8 {
        self.gain_reduction
    }

    pub fn set_dsp_filter_enabled(&mut self, enabled: bool) {
        self.dsp_filter_enabled = enabled;
        if enabled {
            self.recompute_fir();
        } else {
            self.fmac.stop();
        }
    }

    pub fn set_dsp_bandwidth(&mut self, bw: u16) {
        self.bw_hz = bw;
        if self.dsp_filter_enabled {
            self.recompute_fir();
        }
    }

    pub fn set_dsp_shift(&mut self, pbt: i16) {
        self.pbt_hz = pbt;
        if self.dsp_filter_enabled {
            self.recompute_fir();
        }
    }

    pub fn set_cw_peak_enabled(&mut self, enabled: bool) {
        self.cw_peak_enabled = enabled;
        if enabled {
            self.recompute_cw_peak();
        } else {
            self.cw_peak_bq.reset();
        }
    }

    pub fn set_cw_peak_width(&mut self, width: u16) {
        if self.cw_peak_enabled {
            self.cw_peak_bq =
                self.compute_biquad_bandpass(self.cw_pitch_hz as f32, width as f32, SAMPLE_RATE);
        }
    }

    pub fn set_cw_pitch(&mut self, pitch: u16) {
        self.cw_pitch_hz = pitch;
        if self.cw_peak_enabled {
            self.recompute_cw_peak();
        }
    }

    pub fn set_anf_enabled(&mut self, enabled: bool) {
        self.anf_enabled = enabled;
        if !enabled {
            self.anf_weights = [0.0; ANF_TAPS];
            self.anf_history = [0.0; ANF_TAPS + ANF_DELAY];
            self.anf_hist_idx = 0;
        }
    }

    pub fn set_audio_agc_mode(&mut self, mode: AudioAgcMode) {
        self.audio_agc_mode = mode;
        if mode == AudioAgcMode::Off {
            self.agc_envelope = 0.0;
            self.agc_gain = 1.0;
        }
    }

    pub fn set_tx_eq_enabled(&mut self, enabled: bool) {
        self.tx_eq_enabled = enabled;
        if enabled {
            self.recompute_tx_eq();
        } else {
            for bq in &mut self.tx_eq_biquads {
                bq.reset();
            }
        }
    }

    pub fn set_tx_eq_low(&mut self, gain: EqGain) {
        self.tx_eq_low_db = gain.raw();
        if self.tx_eq_enabled {
            self.recompute_tx_eq();
        }
    }

    pub fn set_tx_eq_mid(&mut self, gain: EqGain) {
        self.tx_eq_mid_db = gain.raw();
        if self.tx_eq_enabled {
            self.recompute_tx_eq();
        }
    }

    pub fn set_tx_eq_high(&mut self, gain: EqGain) {
        self.tx_eq_high_db = gain.raw();
        if self.tx_eq_enabled {
            self.recompute_tx_eq();
        }
    }

    pub fn set_rx_eq_enabled(&mut self, enabled: bool) {
        self.rx_eq_enabled = enabled;
        if enabled {
            self.recompute_rx_eq();
        } else {
            for bq in &mut self.rx_eq_biquads {
                bq.reset();
            }
        }
    }

    pub fn set_rx_eq_low(&mut self, gain: EqGain) {
        self.rx_eq_low_db = gain.raw();
        if self.rx_eq_enabled {
            self.recompute_rx_eq();
        }
    }

    pub fn set_rx_eq_mid(&mut self, gain: EqGain) {
        self.rx_eq_mid_db = gain.raw();
        if self.rx_eq_enabled {
            self.recompute_rx_eq();
        }
    }

    pub fn set_rx_eq_high(&mut self, gain: EqGain) {
        self.rx_eq_high_db = gain.raw();
        if self.rx_eq_enabled {
            self.recompute_rx_eq();
        }
    }

    pub fn process_vox(&mut self) -> Option<bool> {
        let result = self.vox.process(&self.mic, &self.rx, self.usb_tx_active);
        if self.vox.is_active() {
            self.mic = result.delayed_mic;
        }
        result.transition
    }

    pub fn set_vox_enabled(&mut self, enabled: bool) {
        self.vox.set_enabled(enabled);
    }

    pub fn set_vox_gain(&mut self, raw: u16) {
        self.vox.set_gain(raw);
    }

    pub fn set_vox_delay(&mut self, ms: u16) {
        self.vox.set_delay(ms);
    }

    pub fn set_vox_anti_trip(&mut self, raw: u16) {
        self.vox.set_anti_trip(raw);
    }

    pub fn set_vox_voice_mode(&mut self, voice: bool) {
        self.vox.set_voice_mode(voice);
    }

    pub fn set_sam_enabled(&mut self, enabled: bool) {
        self.sam_enabled = enabled;
        if !enabled {
            self.sam_pll_phase = 0.0;
            self.sam_pll_freq = 0.0;
        }
    }

    fn apply_sam(&mut self) {
        if !self.sam_enabled {
            return;
        }

        const ALPHA: f32 = 0.0093;
        const BETA: f32 = 0.0000043;
        const MAX_FREQ: f32 = 0.0262;
        let pi = core::f32::consts::PI;

        for sample in &mut self.rx {
            let x = *sample as f32 / 32768.0 - 1.0;

            let (sin_p, cos_p) = self.cordic.sin_cos(self.sam_pll_phase);
            let i = x * cos_p;
            let q = x * (-sin_p);

            let phase_error = if i >= 0.0 { q } else { -q };

            self.sam_pll_freq += BETA * phase_error;
            if self.sam_pll_freq > MAX_FREQ {
                self.sam_pll_freq = MAX_FREQ;
            } else if self.sam_pll_freq < -MAX_FREQ {
                self.sam_pll_freq = -MAX_FREQ;
            }

            self.sam_pll_phase += self.sam_pll_freq + ALPHA * phase_error;
            if self.sam_pll_phase > pi {
                self.sam_pll_phase -= 2.0 * pi;
            } else if self.sam_pll_phase < -pi {
                self.sam_pll_phase += 2.0 * pi;
            }

            *sample = ((i + 1.0) * 32768.0).clamp(0.0, 65535.0) as u16;
        }
    }

    fn compute_fir_bandpass(&mut self, bw_hz: u16, pbt_hz: i16, sr: f32) -> [f32; FIR_TAPS] {
        let mut coeffs = [0.0f32; FIR_TAPS];
        let half = (FIR_TAPS / 2) as f32;
        let bw = bw_hz as f32;
        let shift = pbt_hz as f32;
        let f_low = (shift - bw / 2.0) / sr;
        let f_high = (shift + bw / 2.0) / sr;
        let pi = core::f32::consts::PI;

        let mut sum = 0.0f32;
        for i in 0..FIR_TAPS {
            let n = i as f32 - half + 0.5;
            let sinc_high = if n == 0.0 {
                2.0 * f_high
            } else {
                self.cordic.sinf(2.0 * pi * f_high * n) / (pi * n)
            };
            let sinc_low = if n == 0.0 {
                2.0 * f_low
            } else {
                self.cordic.sinf(2.0 * pi * f_low * n) / (pi * n)
            };
            let window =
                0.54 - 0.46 * self.cordic.cosf(2.0 * pi * i as f32 / (FIR_TAPS as f32 - 1.0));
            coeffs[i] = (sinc_high - sinc_low) * window;
            sum += coeffs[i];
        }

        if sum.abs() > 1e-6 {
            for c in &mut coeffs {
                *c /= sum;
            }
        }
        coeffs
    }

    fn compute_biquad_bandpass(
        &mut self,
        center_hz: f32,
        width_hz: f32,
        sr: f32,
    ) -> BiquadState {
        let pi = core::f32::consts::PI;
        let w0 = 2.0 * pi * center_hz / sr;
        let q = center_hz / width_hz;
        let (sin_w0, cos_w0) = self.cordic.sin_cos(w0);
        let alpha = sin_w0 / (2.0 * q);

        let b0 = alpha;
        let b1 = 0.0;
        let b2 = -alpha;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha;

        BiquadState {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    fn compute_low_shelf(&mut self, freq: f32, gain_db: f32, sr: f32) -> BiquadState {
        let pi = core::f32::consts::PI;
        let a = CordicMath::db_to_amplitude(gain_db as i8);
        let w0 = 2.0 * pi * freq / sr;
        let (sin_w0, cos_w0) = self.cordic.sin_cos(w0);
        let alpha = sin_w0 / 2.0 * CordicMath::sqrt_2();
        let two_sqrt_a_alpha = 2.0 * CordicMath::fast_sqrt(a) * alpha;

        let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha);
        let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha);
        let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha;
        let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha;

        BiquadState {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    fn compute_peak_eq(
        &mut self,
        freq: f32,
        gain_db: f32,
        q: f32,
        sr: f32,
    ) -> BiquadState {
        let pi = core::f32::consts::PI;
        let a = CordicMath::db_to_amplitude(gain_db as i8);
        let w0 = 2.0 * pi * freq / sr;
        let (sin_w0, cos_w0) = self.cordic.sin_cos(w0);
        let alpha = sin_w0 / (2.0 * q);

        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * cos_w0;
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * cos_w0;
        let a2 = 1.0 - alpha / a;

        BiquadState {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    fn compute_high_shelf(&mut self, freq: f32, gain_db: f32, sr: f32) -> BiquadState {
        let pi = core::f32::consts::PI;
        let a = CordicMath::db_to_amplitude(gain_db as i8);
        let w0 = 2.0 * pi * freq / sr;
        let (sin_w0, cos_w0) = self.cordic.sin_cos(w0);
        let alpha = sin_w0 / 2.0 * CordicMath::sqrt_2();
        let two_sqrt_a_alpha = 2.0 * CordicMath::fast_sqrt(a) * alpha;

        let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha);
        let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha);
        let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha;
        let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha;

        BiquadState {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    fn recompute_fir(&mut self) {
        self.fir_coeffs = self.compute_fir_bandpass(self.bw_hz, self.pbt_hz, SAMPLE_RATE);
        self.fmac.stop();
        self.fmac.load_coefficients(&self.fir_coeffs);
        self.fmac.start_fir();
    }

    fn recompute_cw_peak(&mut self) {
        self.cw_peak_bq =
            self.compute_biquad_bandpass(self.cw_pitch_hz as f32, 200.0, SAMPLE_RATE);
    }

    fn recompute_tx_eq(&mut self) {
        self.tx_eq_biquads[0] = self.compute_low_shelf(300.0, self.tx_eq_low_db as f32, SAMPLE_RATE);
        self.tx_eq_biquads[1] =
            self.compute_peak_eq(1000.0, self.tx_eq_mid_db as f32, 1.0, SAMPLE_RATE);
        self.tx_eq_biquads[2] =
            self.compute_high_shelf(3000.0, self.tx_eq_high_db as f32, SAMPLE_RATE);
    }

    fn recompute_rx_eq(&mut self) {
        self.rx_eq_biquads[0] = self.compute_low_shelf(300.0, self.rx_eq_low_db as f32, SAMPLE_RATE);
        self.rx_eq_biquads[1] =
            self.compute_peak_eq(1000.0, self.rx_eq_mid_db as f32, 1.0, SAMPLE_RATE);
        self.rx_eq_biquads[2] =
            self.compute_high_shelf(3000.0, self.rx_eq_high_db as f32, SAMPLE_RATE);
    }

    fn apply_bandpass(&mut self) {
        if !self.dsp_filter_enabled {
            return;
        }
        for sample in &mut self.rx {
            let x_q15 = (*sample as i32 - 32768) as i16;
            let y_q15 = self.fmac.process_sample(x_q15);
            *sample = (y_q15 as i32 + 32768).clamp(0, 65535) as u16;
        }
    }

    fn apply_cw_peak(&mut self) {
        if !self.cw_peak_enabled {
            return;
        }
        for sample in &mut self.rx {
            let x = *sample as f32 / 32768.0 - 1.0;
            let y = self.cw_peak_bq.process(x);
            *sample = ((y + 1.0) * 32768.0).clamp(0.0, 65535.0) as u16;
        }
    }

    fn denoise_rx(&mut self) {
        if !self.nr_enabled {
            return;
        }
        let mu = self.nr_mu as f32 / 32768.0;
        let mut buf = [0.0f32; AUDIO_BUFFER_SIZE];
        for (i, sample) in self.rx.iter().enumerate() {
            buf[i] = *sample as f32 / 32768.0 - 1.0;
        }

        lms_filter(
            &mut buf,
            &mut self.nr_weights,
            &mut self.nr_history,
            &mut self.nr_hist_idx,
            mu,
            NR_TAPS,
            NR_DELAY,
        );

        for (i, sample) in self.rx.iter_mut().enumerate() {
            *sample = ((buf[i] + 1.0) * 32768.0).clamp(0.0, 65535.0) as u16;
        }
    }

    fn denoise_notch(&mut self) {
        if !self.anf_enabled {
            return;
        }
        let mu = 0.0001f32;
        let mut buf = [0.0f32; AUDIO_BUFFER_SIZE];
        for (i, sample) in self.rx.iter().enumerate() {
            buf[i] = *sample as f32 / 32768.0 - 1.0;
        }

        lms_filter(
            &mut buf,
            &mut self.anf_weights,
            &mut self.anf_history,
            &mut self.anf_hist_idx,
            mu,
            ANF_TAPS,
            ANF_DELAY,
        );

        for (i, sample) in self.rx.iter_mut().enumerate() {
            *sample = ((buf[i] + 1.0) * 32768.0).clamp(0.0, 65535.0) as u16;
        }
    }

    fn apply_audio_agc(&mut self) {
        if self.audio_agc_mode == AudioAgcMode::Off {
            return;
        }

        let attack_ms = 5.0f32;
        let release_ms = match self.audio_agc_mode {
            AudioAgcMode::Fast => 500.0,
            AudioAgcMode::Med => 1000.0,
            AudioAgcMode::Slow => 2000.0,
            AudioAgcMode::Off => return,
        };

        let attack_coeff =
            1.0 - CordicMath::fast_exp(-1.0 / (attack_ms * SAMPLE_RATE / 1000.0));
        let release_coeff =
            1.0 - CordicMath::fast_exp(-1.0 / (release_ms * SAMPLE_RATE / 1000.0));
        let max_gain = 31.62f32;
        let noise_gate = 0.001f32;

        for sample in &mut self.rx {
            let x = *sample as f32 / 32768.0 - 1.0;
            let abs_x = if x < 0.0 { -x } else { x };

            if abs_x > self.agc_envelope {
                self.agc_envelope += attack_coeff * (abs_x - self.agc_envelope);
            } else {
                self.agc_envelope += release_coeff * (abs_x - self.agc_envelope);
            }

            if self.agc_envelope < noise_gate {
                self.agc_gain = 1.0;
            } else {
                let target = 0.5 / self.agc_envelope;
                self.agc_gain = if target > max_gain { max_gain } else { target };
            }

            let y = x * self.agc_gain;
            let y_clamped = if y > 1.0 {
                1.0f32
            } else if y < -1.0 {
                -1.0f32
            } else {
                y
            };

            *sample = ((y_clamped + 1.0) * 32768.0).clamp(0.0, 65535.0) as u16;
        }
    }

    fn apply_rx_eq(&mut self) {
        if !self.rx_eq_enabled {
            return;
        }
        for sample in &mut self.rx {
            let mut x = *sample as f32 / 32768.0 - 1.0;
            for bq in &mut self.rx_eq_biquads {
                x = bq.process(x);
            }
            *sample = ((x + 1.0) * 32768.0).clamp(0.0, 65535.0) as u16;
        }
    }

    fn apply_tx_eq(&mut self) {
        if !self.tx_eq_enabled {
            return;
        }
        for sample in &mut self.mic {
            let mut x = *sample as f32 / 32768.0 - 1.0;
            for bq in &mut self.tx_eq_biquads {
                x = bq.process(x);
            }
            *sample = ((x + 1.0) * 32768.0).clamp(0.0, 65535.0) as u16;
        }
    }

    fn compress_mic(&mut self) {
        if self.compression_level == 0 {
            self.gain_reduction = 0;
            return;
        }

        let mut peak: u32 = 0;
        for &sample in &self.mic {
            let signed = (sample as i32) - 32768;
            let abs = signed.unsigned_abs();
            if abs > peak {
                peak = abs;
            }
        }

        let peak_scaled = peak * 256;
        if peak_scaled > self.envelope {
            self.envelope += ATTACK_COEFF * (peak_scaled - self.envelope) / 256;
        } else {
            self.envelope -= RELEASE_COEFF * (self.envelope - peak_scaled) / 256;
        }

        if self.envelope < NOISE_FLOOR * 256 {
            self.gain_reduction = 0;
            return;
        }

        let level = self.compression_level as u32;
        let threshold = 16384 * 256 - (level * 15360 * 256 / 1000);
        let ratio = 2 + (level * 6 / 1000);

        let gain: u32;
        if self.envelope <= threshold {
            gain = 256;
            self.gain_reduction = 0;
        } else {
            let excess = self.envelope - threshold;
            let compressed_excess = excess / ratio;
            let output = threshold + compressed_excess;
            gain = output * 256 / self.envelope;
            self.gain_reduction = (256 - gain).min(255) as u8;
        }

        for sample in &mut self.mic {
            let signed = *sample as i32 - 32768;
            let compressed = signed * gain as i32 / 256;
            *sample = (compressed + 32768).clamp(0, 65535) as u16;
        }
    }

    pub fn mix(&mut self) {
        self.compress_mic();
        self.apply_tx_eq();

        self.apply_bandpass();
        self.apply_sam();
        self.apply_cw_peak();
        self.denoise_rx();
        self.denoise_notch();
        self.apply_audio_agc();
        self.apply_rx_eq();

        if self.usb_tx_active {
            self.usb_tx_timeout += 1;
            if self.usb_tx_timeout > 50 {
                self.usb_tx_active = false;
            }
        }

        let g = self.gains;
        let rx_gain = if self.squelch_open { 255 } else { 0u8 };
        for i in 0..AUDIO_BUFFER_SIZE {
            let rx = scale_u8(self.rx[i], rx_gain);
            let gen = self.generator[i];
            let mic_or_usb = if self.usb_tx_active {
                self.usb_tx[i]
            } else {
                self.mic[i]
            };

            let hp = sat_add(scale_u8(rx, g.rx_to_hp), scale_u8(gen, g.gen_to_hp));
            let spk = scale_u8(
                sat_add(scale_u8(rx, g.rx_to_spk), scale_u8(gen, g.gen_to_spk)),
                self.volume_gain,
            );
            let tx = sat_add(scale_u8(mic_or_usb, g.mic_to_tx), scale_u8(gen, g.gen_to_tx));

            self.out_headphones[i] = if self.headphones_connected { hp } else { 0 };
            self.out_speakers[i] = if !self.headphones_connected { spk } else { 0 };
            self.out_tx[i] = tx;
        }
    }
}

#[inline(always)]
fn scale_u8(sample: u16, gain: u8) -> u16 {
    ((sample as u32 * gain as u32) / 255).min(u16::MAX as u32) as u16
}

#[inline(always)]
fn sat_add(a: u16, b: u16) -> u16 {
    a.saturating_add(b)
}
