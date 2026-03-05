use super::types::{DemodMode, IqBuffer, DSP_BLOCK_SIZE};
use crate::cordic_math::{with_cordic, CordicMutex};

const DC_BLOCK_ALPHA: f32 = 0.995;
const FM_DEVIATION_HZ: f32 = 5000.0;
const FM_NORM: f32 = 48000.0 / (2.0 * core::f32::consts::PI * FM_DEVIATION_HZ);
const SAM_ALPHA: f32 = 0.0093;
const SAM_BETA: f32 = 0.0000043;
const SAM_MAX_FREQ: f32 = 0.0262;

pub struct Demodulator {
    mode: DemodMode,
    am_dc: f32,
    fm_prev_phase: f32,
    sam_pll_phase: f32,
    sam_pll_freq: f32,
    cordic: &'static CordicMutex,
}

impl Demodulator {
    pub fn new(cordic: &'static CordicMutex) -> Self {
        Self {
            mode: DemodMode::Usb,
            am_dc: 0.0,
            fm_prev_phase: 0.0,
            sam_pll_phase: 0.0,
            sam_pll_freq: 0.0,
            cordic,
        }
    }

    pub fn set_mode(&mut self, mode: DemodMode) {
        self.mode = mode;
        self.am_dc = 0.0;
        self.fm_prev_phase = 0.0;
        self.sam_pll_phase = 0.0;
        self.sam_pll_freq = 0.0;
    }

    pub fn process(&mut self, iq: &IqBuffer, audio_out: &mut [f32; DSP_BLOCK_SIZE]) {
        match self.mode {
            DemodMode::Usb | DemodMode::Cw => self.demod_usb(iq, audio_out),
            DemodMode::Lsb => self.demod_lsb(iq, audio_out),
            DemodMode::Am => self.demod_am(iq, audio_out),
            DemodMode::Fm => self.demod_fm(iq, audio_out),
            DemodMode::Sam => self.demod_sam(iq, audio_out),
        }
    }

    fn demod_usb(&self, iq: &IqBuffer, out: &mut [f32; DSP_BLOCK_SIZE]) {
        for i in 0..DSP_BLOCK_SIZE {
            out[i] = iq.i[i];
        }
    }

    fn demod_lsb(&self, iq: &IqBuffer, out: &mut [f32; DSP_BLOCK_SIZE]) {
        for i in 0..DSP_BLOCK_SIZE {
            out[i] = iq.i[i];
        }
    }

    fn demod_am(&mut self, iq: &IqBuffer, out: &mut [f32; DSP_BLOCK_SIZE]) {
        for i in 0..DSP_BLOCK_SIZE {
            let mag_sq = iq.i[i] * iq.i[i] + iq.q[i] * iq.q[i];
            let envelope = with_cordic(self.cordic, |c| c.sqrtf(mag_sq));
            self.am_dc = DC_BLOCK_ALPHA * self.am_dc + (1.0 - DC_BLOCK_ALPHA) * envelope;
            out[i] = envelope - self.am_dc;
        }
    }

    fn demod_fm(&mut self, iq: &IqBuffer, out: &mut [f32; DSP_BLOCK_SIZE]) {
        let pi = core::f32::consts::PI;

        for i in 0..DSP_BLOCK_SIZE {
            let phase = with_cordic(self.cordic, |c| c.atan2f(iq.q[i], iq.i[i]));
            let mut diff = phase - self.fm_prev_phase;
            if diff > pi {
                diff -= 2.0 * pi;
            } else if diff < -pi {
                diff += 2.0 * pi;
            }
            self.fm_prev_phase = phase;

            out[i] = diff * FM_NORM;
        }
    }

    fn demod_sam(&mut self, iq: &IqBuffer, out: &mut [f32; DSP_BLOCK_SIZE]) {
        let pi = core::f32::consts::PI;

        for idx in 0..DSP_BLOCK_SIZE {
            let (sin_p, cos_p) = with_cordic(self.cordic, |c| c.sin_cos(self.sam_pll_phase));

            let i_rot = iq.i[idx] * cos_p + iq.q[idx] * sin_p;
            let q_rot = -iq.i[idx] * sin_p + iq.q[idx] * cos_p;

            let mag = with_cordic(self.cordic, |c| c.sqrtf(i_rot * i_rot + q_rot * q_rot));
            let phase_error = if mag > 1e-6 {
                if i_rot >= 0.0 {
                    q_rot / mag
                } else {
                    -q_rot / mag
                }
            } else {
                0.0
            };

            self.sam_pll_freq += SAM_BETA * phase_error;
            self.sam_pll_freq = self.sam_pll_freq.clamp(-SAM_MAX_FREQ, SAM_MAX_FREQ);

            self.sam_pll_phase += self.sam_pll_freq + SAM_ALPHA * phase_error;
            if self.sam_pll_phase > pi {
                self.sam_pll_phase -= 2.0 * pi;
            } else if self.sam_pll_phase < -pi {
                self.sam_pll_phase += 2.0 * pi;
            }

            self.am_dc = DC_BLOCK_ALPHA * self.am_dc + (1.0 - DC_BLOCK_ALPHA) * i_rot;
            out[idx] = i_rot - self.am_dc;
        }
    }
}
