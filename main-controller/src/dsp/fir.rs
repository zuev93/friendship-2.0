use crate::app::cordic_math::{with_cordic, CordicMutex};

pub const MAX_FIR_TAPS: usize = 255;

pub struct SoftwareFir {
    coeffs: [f32; MAX_FIR_TAPS],
    delay: [f32; MAX_FIR_TAPS],
    taps: usize,
    idx: usize,
}

impl SoftwareFir {
    pub const fn new() -> Self {
        Self {
            coeffs: [0.0; MAX_FIR_TAPS],
            delay: [0.0; MAX_FIR_TAPS],
            taps: 0,
            idx: 0,
        }
    }

    pub fn load_coefficients(&mut self, coeffs: &[f32], num_taps: usize) {
        let n = num_taps.min(MAX_FIR_TAPS);
        self.taps = n;
        self.delay = [0.0; MAX_FIR_TAPS];
        self.idx = 0;
        for i in 0..n {
            self.coeffs[i] = coeffs[i];
        }
    }

    pub fn process_sample(&mut self, input: f32) -> f32 {
        if self.taps == 0 {
            return input;
        }
        self.delay[self.idx] = input;
        let mut acc = 0.0f32;
        let mut di = self.idx;
        for ci in 0..self.taps {
            acc += self.coeffs[ci] * self.delay[di];
            if di == 0 {
                di = self.taps - 1;
            } else {
                di -= 1;
            }
        }
        self.idx += 1;
        if self.idx >= self.taps {
            self.idx = 0;
        }
        acc
    }

    pub fn reset(&mut self) {
        self.delay = [0.0; MAX_FIR_TAPS];
        self.idx = 0;
    }

    pub fn compute_bandpass_coeffs(
        bw_hz: f32,
        shift_hz: f32,
        sample_rate: f32,
        num_taps: usize,
        cordic: &'static CordicMutex,
        out: &mut [f32],
    ) {
        let n = num_taps.min(MAX_FIR_TAPS);
        let half = (n / 2) as f32;
        let f_low = (shift_hz - bw_hz / 2.0) / sample_rate;
        let f_high = (shift_hz + bw_hz / 2.0) / sample_rate;
        let pi = core::f32::consts::PI;

        let mut sum = 0.0f32;
        for i in 0..n {
            let m = i as f32 - half + 0.5;
            let sinc_high = if m == 0.0 {
                2.0 * f_high
            } else {
                with_cordic(cordic, |c| c.sinf(2.0 * pi * f_high * m)) / (pi * m)
            };
            let sinc_low = if m == 0.0 {
                2.0 * f_low
            } else {
                with_cordic(cordic, |c| c.sinf(2.0 * pi * f_low * m)) / (pi * m)
            };
            let x = i as f32 / (n as f32 - 1.0);
            let w = Self::blackman_harris(x, cordic);
            out[i] = (sinc_high - sinc_low) * w;
            sum += out[i];
        }

        if sum.abs() > 1e-6 {
            for c in out[..n].iter_mut() {
                *c /= sum;
            }
        }
    }

    pub fn compute_lowpass_coeffs(
        cutoff_hz: f32,
        sample_rate: f32,
        num_taps: usize,
        cordic: &'static CordicMutex,
        out: &mut [f32],
    ) {
        let n = num_taps.min(MAX_FIR_TAPS);
        let half = (n / 2) as f32;
        let fc = cutoff_hz / sample_rate;
        let pi = core::f32::consts::PI;

        let mut sum = 0.0f32;
        for i in 0..n {
            let m = i as f32 - half + 0.5;
            let sinc = if m == 0.0 {
                2.0 * fc
            } else {
                with_cordic(cordic, |c| c.sinf(2.0 * pi * fc * m)) / (pi * m)
            };
            let x = i as f32 / (n as f32 - 1.0);
            let w = Self::blackman_harris(x, cordic);
            out[i] = sinc * w;
            sum += out[i];
        }

        if sum.abs() > 1e-6 {
            for c in out[..n].iter_mut() {
                *c /= sum;
            }
        }
    }

    fn blackman_harris(x: f32, cordic: &'static CordicMutex) -> f32 {
        let pi2 = 2.0 * core::f32::consts::PI;
        let c1 = with_cordic(cordic, |c| c.cosf(pi2 * x));
        let c2 = with_cordic(cordic, |c| c.cosf(2.0 * pi2 * x));
        let c3 = with_cordic(cordic, |c| c.cosf(3.0 * pi2 * x));
        0.35875 - 0.48829 * c1 + 0.14128 * c2 - 0.01168 * c3
    }
}
