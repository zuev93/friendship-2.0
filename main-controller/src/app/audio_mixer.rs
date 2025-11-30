use crate::consts::AUDIO_BUFFER_SIZE;

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

    pub fn mix(&mut self) {
        let g = self.gains;
        for i in 0..AUDIO_BUFFER_SIZE {
            let rx = self.rx[i];
            let gen = self.generator[i];
            let mic = self.mic[i];

            let hp = sat_add(scale_u8(rx, g.rx_to_hp), scale_u8(gen, g.gen_to_hp));
            let spk = sat_add(scale_u8(rx, g.rx_to_spk), scale_u8(gen, g.gen_to_spk));
            let tx = sat_add(scale_u8(mic, g.mic_to_tx), scale_u8(gen, g.gen_to_tx));

            self.out_headphones[i] = hp;
            self.out_speakers[i] = spk;
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
