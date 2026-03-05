use super::types::{FftResult, FFT_BINS, FFT_SIZE};
use crate::app::cordic_math::{with_cordic, CordicMutex};
use crate::consts::ADC_BUFFER_SIZE;

const HANN_TABLE_SIZE: usize = FFT_SIZE;

pub struct FftEngine {
    twiddle_re: [f32; FFT_BINS],
    twiddle_im: [f32; FFT_BINS],
    window: [f32; HANN_TABLE_SIZE],
    work_re: [f32; FFT_SIZE],
    work_im: [f32; FFT_SIZE],
    avg_bins: [f32; FFT_BINS],
    avg_alpha: f32,
    initialized: bool,
    cordic: &'static CordicMutex,
}

impl FftEngine {
    pub fn new(cordic: &'static CordicMutex) -> Self {
        Self {
            twiddle_re: [0.0; FFT_BINS],
            twiddle_im: [0.0; FFT_BINS],
            window: [0.0; HANN_TABLE_SIZE],
            work_re: [0.0; FFT_SIZE],
            work_im: [0.0; FFT_SIZE],
            avg_bins: [0.0; FFT_BINS],
            avg_alpha: 0.4,
            initialized: false,
            cordic,
        }
    }

    pub fn init(&mut self) {
        let pi = core::f32::consts::PI;

        for k in 0..FFT_BINS {
            let angle = -2.0 * pi * k as f32 / FFT_SIZE as f32;
            let (sin_val, cos_val) = with_cordic(self.cordic, |c| c.sin_cos(angle));
            self.twiddle_re[k] = cos_val;
            self.twiddle_im[k] = sin_val;
        }

        for n in 0..FFT_SIZE {
            let x = n as f32 / (FFT_SIZE as f32 - 1.0);
            let c = with_cordic(self.cordic, |c| c.cosf(2.0 * pi * x));
            self.window[n] = 0.5 * (1.0 - c);
        }

        self.initialized = true;
    }

    pub fn process(&mut self, adc_buffer: &[u32; ADC_BUFFER_SIZE]) -> FftResult {
        if !self.initialized {
            return FftResult::zero();
        }

        let stereo_frames = ADC_BUFFER_SIZE / 2;
        let step = if stereo_frames >= FFT_SIZE {
            stereo_frames / FFT_SIZE
        } else {
            1
        };

        for n in 0..FFT_SIZE {
            let src = (n * step).min(stereo_frames - 1);
            let raw = adc_buffer[src * 2];
            let signed_24 = ((raw << 8) as i32) >> 8;
            let normalized = signed_24 as f32 / 8_388_608.0;
            self.work_re[n] = normalized * self.window[n];
            self.work_im[n] = 0.0;
        }

        self.fft_radix2();

        let mut result = FftResult::zero();
        for k in 0..FFT_BINS {
            let re = self.work_re[k];
            let im = self.work_im[k];
            let mag_sq = re * re + im * im;
            let db = if mag_sq > 1e-20 {
                10.0 * Self::log2_fast(mag_sq) * 0.30103
            } else {
                -120.0
            };

            self.avg_bins[k] = self.avg_alpha * db + (1.0 - self.avg_alpha) * self.avg_bins[k];
            result.bins[k] = self.avg_bins[k];
        }

        result
    }

    fn fft_radix2(&mut self) {
        let n = FFT_SIZE;
        let log2n = 10;

        let mut j = 0usize;
        for i in 0..n {
            if i < j {
                let tmp_re = self.work_re[i];
                let tmp_im = self.work_im[i];
                self.work_re[i] = self.work_re[j];
                self.work_im[i] = self.work_im[j];
                self.work_re[j] = tmp_re;
                self.work_im[j] = tmp_im;
            }
            let mut m = n >> 1;
            while m >= 1 && j >= m {
                j -= m;
                m >>= 1;
            }
            j += m;
        }

        let mut stage_len = 2;
        for _ in 0..log2n {
            let half = stage_len / 2;
            let tw_step = n / stage_len;

            let mut group_start = 0;
            while group_start < n {
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
                    self.work_re[even] = self.work_re[even] + t_re;
                    self.work_im[even] = self.work_im[even] + t_im;
                }
                group_start += stage_len;
            }
            stage_len <<= 1;
        }
    }

    fn log2_fast(x: f32) -> f32 {
        if x <= 0.0 {
            return -40.0;
        }
        let bits = x.to_bits();
        let exp = ((bits >> 23) & 0xFF) as f32 - 127.0;
        let mant = f32::from_bits((bits & 0x007F_FFFF) | 0x3F80_0000);
        exp + (mant - 1.0) * 1.4427
    }
}
