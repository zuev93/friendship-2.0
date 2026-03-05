use crate::{
    app::{
        cordic_math::{with_cordic, CordicMath, CordicMutex},
        types::{Compression, EqGain, NrLevel, Volume},
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
    anf_enabled: bool,
    anf_weights: [f32; ANF_TAPS],
    anf_history: [f32; ANF_TAPS + ANF_DELAY],
    anf_hist_idx: usize,
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
    cordic: &'static CordicMutex,
    vox: VoxProcessor,
}

impl AudioMixer {
    pub fn new(cordic: &'static CordicMutex) -> Self {
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
            anf_enabled: false,
            anf_weights: [0.0; ANF_TAPS],
            anf_history: [0.0; ANF_TAPS + ANF_DELAY],
            anf_hist_idx: 0,
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
            cordic,
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

    pub fn set_anf_enabled(&mut self, enabled: bool) {
        self.anf_enabled = enabled;
        if !enabled {
            self.anf_weights = [0.0; ANF_TAPS];
            self.anf_history = [0.0; ANF_TAPS + ANF_DELAY];
            self.anf_hist_idx = 0;
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

    fn compute_low_shelf(&mut self, freq: f32, gain_db: f32, sr: f32) -> BiquadState {
        let pi = core::f32::consts::PI;
        let a = with_cordic(self.cordic, |c| c.db_to_amplitude(gain_db));
        let w0 = 2.0 * pi * freq / sr;
        let (sin_w0, cos_w0) = with_cordic(self.cordic, |c| c.sin_cos(w0));
        let alpha = sin_w0 / 2.0 * CordicMath::sqrt_2();
        let sqrt_a = with_cordic(self.cordic, |c| c.sqrtf(a.clamp(0.027, 0.75)));
        let two_sqrt_a_alpha = 2.0 * sqrt_a * alpha;

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

    fn compute_peak_eq(&mut self, freq: f32, gain_db: f32, q: f32, sr: f32) -> BiquadState {
        let pi = core::f32::consts::PI;
        let a = with_cordic(self.cordic, |c| c.db_to_amplitude(gain_db));
        let w0 = 2.0 * pi * freq / sr;
        let (sin_w0, cos_w0) = with_cordic(self.cordic, |c| c.sin_cos(w0));
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
        let a = with_cordic(self.cordic, |c| c.db_to_amplitude(gain_db));
        let w0 = 2.0 * pi * freq / sr;
        let (sin_w0, cos_w0) = with_cordic(self.cordic, |c| c.sin_cos(w0));
        let alpha = sin_w0 / 2.0 * CordicMath::sqrt_2();
        let sqrt_a = with_cordic(self.cordic, |c| c.sqrtf(a.clamp(0.027, 0.75)));
        let two_sqrt_a_alpha = 2.0 * sqrt_a * alpha;

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

    fn recompute_tx_eq(&mut self) {
        self.tx_eq_biquads[0] =
            self.compute_low_shelf(300.0, self.tx_eq_low_db as f32, SAMPLE_RATE);
        self.tx_eq_biquads[1] =
            self.compute_peak_eq(1000.0, self.tx_eq_mid_db as f32, 1.0, SAMPLE_RATE);
        self.tx_eq_biquads[2] =
            self.compute_high_shelf(3000.0, self.tx_eq_high_db as f32, SAMPLE_RATE);
    }

    fn recompute_rx_eq(&mut self) {
        self.rx_eq_biquads[0] =
            self.compute_low_shelf(300.0, self.rx_eq_low_db as f32, SAMPLE_RATE);
        self.rx_eq_biquads[1] =
            self.compute_peak_eq(1000.0, self.rx_eq_mid_db as f32, 1.0, SAMPLE_RATE);
        self.rx_eq_biquads[2] =
            self.compute_high_shelf(3000.0, self.rx_eq_high_db as f32, SAMPLE_RATE);
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

        Self::lms_filter(
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

        Self::lms_filter(
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

    #[inline(always)]
    fn scale_u8(sample: u16, gain: u8) -> u16 {
        ((sample as u32 * gain as u32) / 255).min(u16::MAX as u32) as u16
    }

    #[inline(always)]
    fn sat_add(a: u16, b: u16) -> u16 {
        a.saturating_add(b)
    }

    pub fn mix(&mut self) {
        self.compress_mic();
        self.apply_tx_eq();

        self.denoise_rx();
        self.denoise_notch();
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
            let rx = Self::scale_u8(self.rx[i], rx_gain);
            let gen = self.generator[i];
            let mic_or_usb = if self.usb_tx_active {
                self.usb_tx[i]
            } else {
                self.mic[i]
            };

            let hp = Self::sat_add(
                Self::scale_u8(rx, g.rx_to_hp),
                Self::scale_u8(gen, g.gen_to_hp),
            );
            let spk = Self::scale_u8(
                Self::sat_add(
                    Self::scale_u8(rx, g.rx_to_spk),
                    Self::scale_u8(gen, g.gen_to_spk),
                ),
                self.volume_gain,
            );
            let tx = Self::sat_add(
                Self::scale_u8(mic_or_usb, g.mic_to_tx),
                Self::scale_u8(gen, g.gen_to_tx),
            );

            self.out_headphones[i] = if self.headphones_connected { hp } else { 0 };
            self.out_speakers[i] = if !self.headphones_connected { spk } else { 0 };
            self.out_tx[i] = tx;
        }
    }
}
