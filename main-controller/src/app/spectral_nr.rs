use crate::consts::AUDIO_BUFFER_SIZE;
use crate::cordic_math::{with_cordic, CordicMutex};

const FFT_N: usize = AUDIO_BUFFER_SIZE;
const FFT_LOG2: usize = 9;
const BINS: usize = FFT_N / 2 + 1;
const HOP: usize = FFT_N / 2;

const NOISE_DECAY: f32 = 1.002;
const DD_ALPHA: f32 = 0.98;
const GAIN_MIN_DEFAULT: f32 = 0.2;
const GAIN_MIN_LOW: f32 = 0.5;
const GAIN_MIN_HIGH: f32 = 0.02;

pub struct SpectralNr {
    twiddle_re: [f32; FFT_N / 2],
    twiddle_im: [f32; FFT_N / 2],
    hann: [f32; FFT_N],
    noise_est: [f32; BINS],
    prev_clean_power: [f32; BINS],
    work_re: [f32; FFT_N],
    work_im: [f32; FFT_N],
    prev_input: [f32; HOP],
    overlap_tail: [f32; HOP],
    gain_min: f32,
    initialized: bool,
    cordic: &'static CordicMutex,
}

impl SpectralNr {
    pub fn new(cordic: &'static CordicMutex) -> Self {
        Self {
            twiddle_re: [0.0; FFT_N / 2],
            twiddle_im: [0.0; FFT_N / 2],
            hann: [0.0; FFT_N],
            noise_est: [0.0; BINS],
            prev_clean_power: [0.0; BINS],
            work_re: [0.0; FFT_N],
            work_im: [0.0; FFT_N],
            prev_input: [0.0; HOP],
            overlap_tail: [0.0; HOP],
            gain_min: GAIN_MIN_DEFAULT,
            initialized: false,
            cordic,
        }
    }

    pub fn set_level(&mut self, raw_0_1000: i16) {
        let t = raw_0_1000.clamp(0, 1000) as f32 / 1000.0;
        self.gain_min = GAIN_MIN_LOW + t * (GAIN_MIN_HIGH - GAIN_MIN_LOW);
    }

    fn ensure_init(&mut self) {
        if self.initialized {
            return;
        }
        let pi = core::f32::consts::PI;
        for k in 0..FFT_N / 2 {
            let angle = -2.0 * pi * k as f32 / FFT_N as f32;
            let (sin_val, cos_val) = with_cordic(self.cordic, |c| c.sin_cos(angle));
            self.twiddle_re[k] = cos_val;
            self.twiddle_im[k] = sin_val;
        }
        for n in 0..FFT_N {
            let angle = 2.0 * pi * n as f32 / FFT_N as f32;
            let (sin_val, cos_val) = with_cordic(self.cordic, |c| c.sin_cos(angle));
            let _ = sin_val;
            self.hann[n] = 0.5 * (1.0 - cos_val);
        }
        self.initialized = true;
    }

    pub fn process(&mut self, buffer: &mut [f32; AUDIO_BUFFER_SIZE]) {
        self.ensure_init();

        let mut output = [0.0f32; AUDIO_BUFFER_SIZE];

        let mut frame1_input = [0.0f32; FFT_N];
        frame1_input[..HOP].copy_from_slice(&self.prev_input);
        frame1_input[HOP..].copy_from_slice(&buffer[..HOP]);
        self.process_subframe(&frame1_input);
        for i in 0..HOP {
            output[i] = self.overlap_tail[i] + self.work_re[i];
        }
        self.overlap_tail.copy_from_slice(&self.work_re[HOP..FFT_N]);

        let mut frame2_input = [0.0f32; FFT_N];
        frame2_input[..HOP].copy_from_slice(&buffer[..HOP]);
        frame2_input[HOP..].copy_from_slice(&buffer[HOP..]);
        self.process_subframe(&frame2_input);
        for i in 0..HOP {
            output[HOP + i] = self.overlap_tail[i] + self.work_re[i];
        }
        self.overlap_tail.copy_from_slice(&self.work_re[HOP..FFT_N]);

        self.prev_input.copy_from_slice(&buffer[HOP..]);

        buffer.copy_from_slice(&output);
    }

    fn process_subframe(&mut self, input: &[f32; FFT_N]) {
        for i in 0..FFT_N {
            self.work_re[i] = input[i] * self.hann[i];
            self.work_im[i] = 0.0;
        }

        self.fft_forward();
        self.wiener_filter();
        self.fft_inverse();

        for i in 0..FFT_N {
            self.work_re[i] *= self.hann[i];
        }
    }

    fn wiener_filter(&mut self) {
        let gain_min = self.gain_min;
        let gain_min_sq = gain_min * gain_min;

        for k in 0..BINS {
            let re = self.work_re[k];
            let im = self.work_im[k];
            let noisy_power = re * re + im * im;

            if self.noise_est[k] < 1e-10 {
                self.noise_est[k] = noisy_power;
            } else {
                self.noise_est[k] = (self.noise_est[k] * NOISE_DECAY).min(noisy_power);
            }

            let noise = self.noise_est[k];

            let gamma = if noise > 1e-10 {
                noisy_power / noise
            } else {
                1000.0
            };

            let gamma_m1 = (gamma - 1.0).max(0.0);
            let dd_prior = if noise > 1e-10 {
                self.prev_clean_power[k] / noise
            } else {
                0.0
            };
            let xi = DD_ALPHA * dd_prior + (1.0 - DD_ALPHA) * gamma_m1;

            let gain_sq = (xi / (1.0 + xi)).max(gain_min_sq);
            let gain = with_cordic(self.cordic, |c| c.sqrtf(gain_sq));

            let clean_re = re * gain;
            let clean_im = im * gain;
            self.prev_clean_power[k] = clean_re * clean_re + clean_im * clean_im;

            self.work_re[k] = clean_re;
            self.work_im[k] = clean_im;

            if k > 0 && k < FFT_N / 2 {
                self.work_re[FFT_N - k] *= gain;
                self.work_im[FFT_N - k] *= gain;
            }
        }
    }

    fn fft_forward(&mut self) {
        self.bit_reverse();
        self.butterfly();
    }

    fn fft_inverse(&mut self) {
        for i in 0..FFT_N {
            self.work_im[i] = -self.work_im[i];
        }
        self.bit_reverse();
        self.butterfly();
        let scale = 1.0 / FFT_N as f32;
        for i in 0..FFT_N {
            self.work_re[i] *= scale;
            self.work_im[i] = -self.work_im[i] * scale;
        }
    }

    fn bit_reverse(&mut self) {
        let mut j = 0usize;
        for i in 0..FFT_N {
            if i < j {
                let tmp_re = self.work_re[i];
                let tmp_im = self.work_im[i];
                self.work_re[i] = self.work_re[j];
                self.work_im[i] = self.work_im[j];
                self.work_re[j] = tmp_re;
                self.work_im[j] = tmp_im;
            }
            let mut m = FFT_N >> 1;
            while m >= 1 && j >= m {
                j -= m;
                m >>= 1;
            }
            j += m;
        }
    }

    fn butterfly(&mut self) {
        let mut stage_len = 2;
        for _ in 0..FFT_LOG2 {
            let half = stage_len / 2;
            let tw_step = FFT_N / stage_len;

            let mut group_start = 0;
            while group_start < FFT_N {
                for k in 0..half {
                    let tw_idx = k * tw_step;
                    let tw_re = self.twiddle_re[tw_idx];
                    let tw_im = self.twiddle_im[tw_idx];

                    let even = group_start + k;
                    let odd = group_start + k + half;

                    let odd_re = self.work_re[odd];
                    let odd_im = self.work_im[odd];

                    let t_re = tw_re * odd_re - tw_im * odd_im;
                    let t_im = tw_re * odd_im + tw_im * odd_re;

                    self.work_re[odd] = self.work_re[even] - t_re;
                    self.work_im[odd] = self.work_im[even] - t_im;
                    self.work_re[even] += t_re;
                    self.work_im[even] += t_im;
                }
                group_start += stage_len;
            }
            stage_len <<= 1;
        }
    }
}
