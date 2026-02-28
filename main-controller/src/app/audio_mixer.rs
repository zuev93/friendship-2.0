use crate::{app::types::Volume, consts::AUDIO_BUFFER_SIZE};

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
}

impl AudioMixer {
    pub const fn new() -> Self {
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

    pub fn mix(&mut self) {
        let g = self.gains;
        let rx_gain = if self.squelch_open { 255 } else { 0u8 };
        for i in 0..AUDIO_BUFFER_SIZE {
            let rx = scale_u8(self.rx[i], rx_gain);
            let gen = self.generator[i];
            let mic = self.mic[i];

            let hp = sat_add(scale_u8(rx, g.rx_to_hp), scale_u8(gen, g.gen_to_hp));
            let spk = scale_u8(
                sat_add(scale_u8(rx, g.rx_to_spk), scale_u8(gen, g.gen_to_spk)),
                self.volume_gain,
            );
            let tx = sat_add(scale_u8(mic, g.mic_to_tx), scale_u8(gen, g.gen_to_tx));

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
