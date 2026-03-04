use super::types::{AgcPreset, IqBuffer, DSP_BLOCK_SIZE};
use crate::app::cordic_math::{with_cordic, CordicMutex};
use crate::consts::DSP_SAMPLE_RATE;

const MAX_GAIN_DB: f32 = 80.0;
const NOISE_GATE: f32 = 1e-6;
const TARGET_LEVEL: f32 = 0.5;

pub struct DigitalAgc {
    preset: AgcPreset,
    envelope: f32,
    gain: f32,
    hang_counter: u32,
    hang_samples: u32,
    attack_coeff: f32,
    release_coeff: f32,
    max_gain: f32,
    manual_gain: f32,
    cordic: &'static CordicMutex,
    current_level_db: f32,
}

impl DigitalAgc {
    pub fn new(cordic: &'static CordicMutex) -> Self {
        let mut agc = Self {
            preset: AgcPreset::Off,
            envelope: 0.0,
            gain: 1.0,
            hang_counter: 0,
            hang_samples: 0,
            attack_coeff: 0.0,
            release_coeff: 0.0,
            max_gain: 1.0,
            manual_gain: 1.0,
            cordic,
            current_level_db: -120.0,
        };
        agc.set_preset(AgcPreset::SsbFast);
        agc
    }

    pub fn set_preset(&mut self, preset: AgcPreset) {
        self.preset = preset;
        if preset == AgcPreset::Off {
            self.gain = 1.0;
            self.envelope = 0.0;
            return;
        }
        if preset == AgcPreset::Manual {
            return;
        }
        let sr = DSP_SAMPLE_RATE as f32;
        let attack_ms = preset.attack_ms();
        let release_ms = preset.release_ms();
        let hang_ms = preset.hang_ms();

        self.attack_coeff = if attack_ms > 0.0 {
            1.0 - with_cordic(self.cordic, |c| c.expf(-1.0 / (attack_ms * sr / 1000.0)))
        } else {
            1.0
        };
        self.release_coeff = if release_ms > 0.0 {
            1.0 - with_cordic(self.cordic, |c| c.expf(-1.0 / (release_ms * sr / 1000.0)))
        } else {
            0.01
        };
        self.hang_samples = (hang_ms * sr / 1000.0) as u32;
        self.max_gain = with_cordic(self.cordic, |c| c.db_to_amplitude(MAX_GAIN_DB));
        self.envelope = 0.0;
        self.gain = 1.0;
        self.hang_counter = 0;
    }

    pub fn set_manual_gain_db(&mut self, db: f32) {
        self.manual_gain = with_cordic(self.cordic, |c| c.db_to_amplitude(db.clamp(-20.0, 80.0)));
    }

    pub fn process(&mut self, iq: &mut IqBuffer) {
        match self.preset {
            AgcPreset::Off => {}
            AgcPreset::Manual => {
                let g = self.manual_gain;
                for i in 0..DSP_BLOCK_SIZE {
                    iq.i[i] *= g;
                    iq.q[i] *= g;
                }
            }
            _ => self.process_auto(iq),
        }
    }

    fn process_auto(&mut self, iq: &mut IqBuffer) {
        for idx in 0..DSP_BLOCK_SIZE {
            let mag_sq = iq.i[idx] * iq.i[idx] + iq.q[idx] * iq.q[idx];
            let mag = fast_sqrt(mag_sq);

            if mag > self.envelope {
                self.envelope += self.attack_coeff * (mag - self.envelope);
                self.hang_counter = self.hang_samples;
            } else if self.hang_counter > 0 {
                self.hang_counter -= 1;
            } else {
                self.envelope += self.release_coeff * (mag - self.envelope);
            }

            if self.envelope > NOISE_GATE {
                let target_gain = TARGET_LEVEL / self.envelope;
                self.gain = if target_gain > self.max_gain {
                    self.max_gain
                } else {
                    target_gain
                };
            } else {
                self.gain = 1.0;
            }

            iq.i[idx] *= self.gain;
            iq.q[idx] *= self.gain;
        }

        if self.envelope > NOISE_GATE {
            self.current_level_db = 20.0 * log2_approx(self.envelope) * 0.30103;
        } else {
            self.current_level_db = -120.0;
        }
    }

    pub fn current_level_db(&self) -> f32 {
        self.current_level_db
    }

    pub fn current_gain(&self) -> f32 {
        self.gain
    }
}

pub struct AnalogAgc {
    target_dbfs: f32,
    current_gain_dac: u16,
    attack_step: u16,
    release_step: u16,
}

impl AnalogAgc {
    pub const fn new() -> Self {
        Self {
            target_dbfs: -12.0,
            current_gain_dac: 2048,
            attack_step: 100,
            release_step: 10,
        }
    }

    pub fn process_adc_peak(&mut self, peak_24bit: i32) -> u16 {
        let dbfs = if peak_24bit > 0 {
            20.0 * log2_approx(peak_24bit as f32 / 8_388_607.0) * 0.30103
        } else {
            -120.0
        };

        if dbfs > -3.0 {
            self.current_gain_dac = self.current_gain_dac.saturating_sub(self.attack_step);
        } else if dbfs < -20.0 {
            self.current_gain_dac = (self.current_gain_dac + self.release_step).min(4095);
        }

        self.current_gain_dac
    }

    pub fn gain_db(&self) -> f32 {
        (self.current_gain_dac as f32 / 4095.0) * 45.0 - 2.5
    }
}

fn fast_sqrt(x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    let mut y = f32::from_bits((x.to_bits() >> 1) + 0x1FC0_0000);
    y = 0.5 * (y + x / y);
    y = 0.5 * (y + x / y);
    y
}

fn log2_approx(x: f32) -> f32 {
    if x <= 0.0 {
        return -120.0;
    }
    let bits = x.to_bits();
    let exp = ((bits >> 23) & 0xFF) as f32 - 127.0;
    let mant = f32::from_bits((bits & 0x007F_FFFF) | 0x3F80_0000);
    exp + (mant - 1.0) * 1.4427
}
