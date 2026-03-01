use crate::{
    app::types::{Compression, NrLevel, Volume},
    consts::AUDIO_BUFFER_SIZE,
};

const NR_TAPS: usize = 64;
const NR_DELAY: usize = 1;

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
    nr_weights: [i32; NR_TAPS],
    nr_history: [i16; NR_TAPS + NR_DELAY],
    nr_hist_idx: usize,
    usb_tx: [u16; AUDIO_BUFFER_SIZE],
    usb_tx_active: bool,
    usb_tx_timeout: u16,
}

impl AudioMixer {
    pub fn new() -> Self {
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
            nr_weights: [0; NR_TAPS],
            nr_history: [0; NR_TAPS + NR_DELAY],
            nr_hist_idx: 0,
            usb_tx: [32768; AUDIO_BUFFER_SIZE],
            usb_tx_active: false,
            usb_tx_timeout: 0,
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
            self.nr_weights = [0; NR_TAPS];
            self.nr_history = [0; NR_TAPS + NR_DELAY];
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

    fn denoise_rx(&mut self) {
        if !self.nr_enabled {
            return;
        }
        let mu = self.nr_mu as i32;
        let hist_len = NR_TAPS + NR_DELAY;

        for sample in &mut self.rx {
            let x = *sample as i32 - 32768;

            self.nr_history[self.nr_hist_idx] = x as i16;
            self.nr_hist_idx = (self.nr_hist_idx + 1) % hist_len;

            let mut y: i32 = 0;
            for k in 0..NR_TAPS {
                let delay_idx =
                    (self.nr_hist_idx + hist_len - NR_DELAY - 1 - k) % hist_len;
                let delayed = self.nr_history[delay_idx] as i32;
                y += (self.nr_weights[k] * delayed) >> 15;
            }

            let error = x - y;

            for k in 0..NR_TAPS {
                let delay_idx =
                    (self.nr_hist_idx + hist_len - NR_DELAY - 1 - k) % hist_len;
                let delayed = self.nr_history[delay_idx] as i32;
                self.nr_weights[k] += (mu * error * delayed) >> 25;
            }

            *sample = (error + 32768).clamp(0, 65535) as u16;
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
        self.denoise_rx();

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
