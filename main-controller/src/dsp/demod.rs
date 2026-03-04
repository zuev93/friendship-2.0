use super::types::{DemodMode, IqBuffer, DSP_BLOCK_SIZE};
use crate::app::cordic_math::{with_cordic, CordicMutex};

const SAM_ALPHA: f32 = 0.0093;
const SAM_BETA: f32 = 0.0000043;
const SAM_MAX_FREQ: f32 = 0.0262;
const DC_BLOCK_ALPHA: f32 = 0.995;
const FM_DEEMPH_ALPHA: f32 = 0.072;

pub struct Demodulator {
    mode: DemodMode,
    sam_pll_phase: f32,
    sam_pll_freq: f32,
    am_dc: f32,
    fm_prev_phase: f32,
    cordic: &'static CordicMutex,
}

impl Demodulator {
    pub fn new(cordic: &'static CordicMutex) -> Self {
        Self {
            mode: DemodMode::Usb,
            sam_pll_phase: 0.0,
            sam_pll_freq: 0.0,
            am_dc: 0.0,
            fm_prev_phase: 0.0,
            cordic,
        }
    }

    pub fn set_mode(&mut self, mode: DemodMode) {
        self.mode = mode;
        self.sam_pll_phase = 0.0;
        self.sam_pll_freq = 0.0;
        self.am_dc = 0.0;
        self.fm_prev_phase = 0.0;
    }

    pub fn mode(&self) -> DemodMode {
        self.mode
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
            let envelope = fast_sqrt(mag_sq);

            self.am_dc = DC_BLOCK_ALPHA * self.am_dc + (1.0 - DC_BLOCK_ALPHA) * envelope;
            out[i] = envelope - self.am_dc;
        }
    }

    fn demod_fm(&mut self, iq: &IqBuffer, out: &mut [f32; DSP_BLOCK_SIZE]) {
        let mut prev_out = 0.0f32;

        for i in 0..DSP_BLOCK_SIZE {
            let phase = with_cordic(self.cordic, |c| {
                let angle = if iq.i[i].abs() > 1e-12 {
                    let ratio = iq.q[i] / iq.i[i];
                    c.sinf(0.0) * 0.0 + ratio.clamp(-1.0, 1.0)
                } else if iq.q[i] >= 0.0 {
                    1.0
                } else {
                    -1.0
                };
                angle
            });

            let atan_approx = fast_atan2(iq.q[i], iq.i[i]);
            let mut diff = atan_approx - self.fm_prev_phase;
            if diff > core::f32::consts::PI {
                diff -= 2.0 * core::f32::consts::PI;
            } else if diff < -core::f32::consts::PI {
                diff += 2.0 * core::f32::consts::PI;
            }
            self.fm_prev_phase = atan_approx;

            let demod = diff / core::f32::consts::PI;
            let filtered = FM_DEEMPH_ALPHA * demod + (1.0 - FM_DEEMPH_ALPHA) * prev_out;
            prev_out = filtered;
            out[i] = filtered;
        }
    }

    fn demod_sam(&mut self, iq: &IqBuffer, out: &mut [f32; DSP_BLOCK_SIZE]) {
        let pi = core::f32::consts::PI;

        for idx in 0..DSP_BLOCK_SIZE {
            let (sin_p, cos_p) = with_cordic(self.cordic, |c| c.sin_cos(self.sam_pll_phase));

            let i_rot = iq.i[idx] * cos_p + iq.q[idx] * sin_p;
            let q_rot = -iq.i[idx] * sin_p + iq.q[idx] * cos_p;

            let phase_error = if i_rot >= 0.0 { q_rot } else { -q_rot };

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

fn fast_sqrt(x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    let mut y = f32::from_bits((x.to_bits() >> 1) + 0x1FC0_0000);
    y = 0.5 * (y + x / y);
    y
}

fn fast_atan2(y: f32, x: f32) -> f32 {
    let pi = core::f32::consts::PI;
    if x == 0.0 && y == 0.0 {
        return 0.0;
    }
    let abs_x = if x < 0.0 { -x } else { x };
    let abs_y = if y < 0.0 { -y } else { y };

    let (a, offset) = if abs_x >= abs_y {
        let r = abs_y / abs_x;
        (r * (0.7854 - 0.2146 * r), 0.0)
    } else {
        let r = abs_x / abs_y;
        (pi / 2.0 - r * (0.7854 - 0.2146 * r), 0.0)
    };

    let angle = a + offset;
    let angle = if x < 0.0 { pi - angle } else { angle };
    if y < 0.0 {
        -angle
    } else {
        angle
    }
}
