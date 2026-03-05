use super::fir::{SoftwareFir, MAX_FIR_TAPS};
use super::types::{DemodMode, DSP_BLOCK_SIZE};
use crate::consts::{ADC_BUFFER_SIZE, ADC_SAMPLE_RATE, DSP_SAMPLE_RATE};
use crate::cordic_math::{with_cordic, CordicMutex};

const TX_INTERP_TAPS: usize = 63;
const CW_ENVELOPE_MS: f32 = 5.0;
const TX_IF_HZ: f32 = 50_000.0;
const TX_HPF_ALPHA: f32 = 0.962;
const TX_COMP_THRESHOLD: f32 = 0.25;
const TX_COMP_RATIO: f32 = 0.25;
const TX_COMP_ATTACK: f32 = 0.004;
const TX_COMP_RELEASE: f32 = 0.0004;
const TX_LIMITER_CEIL: f32 = 0.95;
const FM_DEVIATION_HZ: f32 = 5000.0;

pub struct TxModulator {
    mode: DemodMode,
    hilbert_fir: SoftwareFir,
    delay_line: [f32; 32],
    delay_idx: usize,
    interp_fir_i: SoftwareFir,
    interp_fir_q: SoftwareFir,
    cordic: &'static CordicMutex,
    nco_phase: f32,
    nco_phase_step: f32,
    cw_envelope: f32,
    cw_target_envelope: f32,
    cw_ramp_step: f32,
    hpf_prev_in: f32,
    hpf_prev_out: f32,
    comp_envelope: f32,
    compressor_enabled: bool,
    fm_mod_phase: f32,
}

impl TxModulator {
    pub fn new(cordic: &'static CordicMutex) -> Self {
        let ramp_samples = CW_ENVELOPE_MS * DSP_SAMPLE_RATE as f32 / 1000.0;
        let nco_step = 2.0 * core::f32::consts::PI * TX_IF_HZ / ADC_SAMPLE_RATE as f32;
        let mut tx = Self {
            mode: DemodMode::Usb,
            hilbert_fir: SoftwareFir::new(),
            delay_line: [0.0; 32],
            delay_idx: 0,
            interp_fir_i: SoftwareFir::new(),
            interp_fir_q: SoftwareFir::new(),
            cordic,
            nco_phase: 0.0,
            nco_phase_step: nco_step,
            cw_envelope: 0.0,
            cw_target_envelope: 0.0,
            cw_ramp_step: 1.0 / ramp_samples,
            hpf_prev_in: 0.0,
            hpf_prev_out: 0.0,
            comp_envelope: 0.0,
            compressor_enabled: true,
            fm_mod_phase: 0.0,
        };
        tx.init_hilbert();
        tx.init_interp_filters();
        tx
    }

    pub fn set_mode(&mut self, mode: DemodMode) {
        self.mode = mode;
        self.hpf_prev_in = 0.0;
        self.hpf_prev_out = 0.0;
        self.comp_envelope = 0.0;
        self.fm_mod_phase = 0.0;
    }

    pub fn set_cw_pitch(&mut self, _pitch_hz: u16) {}

    pub fn set_cw_key(&mut self, down: bool) {
        self.cw_target_envelope = if down { 1.0 } else { 0.0 };
    }

    pub fn set_compressor_enabled(&mut self, enabled: bool) {
        self.compressor_enabled = enabled;
    }

    pub fn process(
        &mut self,
        audio_in: &[f32; DSP_BLOCK_SIZE],
        dac_out: &mut [u32; ADC_BUFFER_SIZE],
    ) {
        match self.mode {
            DemodMode::Usb => self.modulate_ssb(audio_in, dac_out, false),
            DemodMode::Sam => self.modulate_am(audio_in, dac_out),
            DemodMode::Lsb => self.modulate_ssb(audio_in, dac_out, true),
            DemodMode::Cw => self.modulate_cw(dac_out),
            DemodMode::Am => self.modulate_am(audio_in, dac_out),
            DemodMode::Fm => self.modulate_fm(audio_in, dac_out),
        }
    }

    fn apply_tx_processing(&mut self, sample: f32) -> f32 {
        let hpf_out = TX_HPF_ALPHA * (self.hpf_prev_out + sample - self.hpf_prev_in);
        self.hpf_prev_in = sample;
        self.hpf_prev_out = hpf_out;

        if !self.compressor_enabled {
            return hpf_out;
        }

        let abs_val = if hpf_out < 0.0 { -hpf_out } else { hpf_out };
        let coeff = if abs_val > self.comp_envelope {
            TX_COMP_ATTACK
        } else {
            TX_COMP_RELEASE
        };
        self.comp_envelope += coeff * (abs_val - self.comp_envelope);

        let gain = if self.comp_envelope > TX_COMP_THRESHOLD {
            (TX_COMP_THRESHOLD + (self.comp_envelope - TX_COMP_THRESHOLD) * TX_COMP_RATIO)
                / self.comp_envelope
        } else {
            1.0
        };

        (hpf_out * gain).clamp(-TX_LIMITER_CEIL, TX_LIMITER_CEIL)
    }

    fn modulate_ssb(
        &mut self,
        audio: &[f32; DSP_BLOCK_SIZE],
        dac_out: &mut [u32; ADC_BUFFER_SIZE],
        invert_q: bool,
    ) {
        let mut i_buf = [0.0f32; DSP_BLOCK_SIZE];
        let mut q_buf = [0.0f32; DSP_BLOCK_SIZE];

        for idx in 0..DSP_BLOCK_SIZE {
            let processed = self.apply_tx_processing(audio[idx]);

            self.delay_line[self.delay_idx] = processed;
            self.delay_idx = (self.delay_idx + 1) % 32;
            let delay_read = (self.delay_idx + 32 - 16) % 32;
            i_buf[idx] = self.delay_line[delay_read];
            q_buf[idx] = self.hilbert_fir.process_sample(processed);
        }

        if invert_q {
            for s in q_buf.iter_mut() {
                *s = -*s;
            }
        }

        self.upsample_iq(&i_buf, &q_buf, dac_out);
    }

    fn modulate_cw(&mut self, dac_out: &mut [u32; ADC_BUFFER_SIZE]) {
        let pi = core::f32::consts::PI;
        let stereo_frames = ADC_BUFFER_SIZE / 2;
        let ramp = self.cw_ramp_step * (DSP_SAMPLE_RATE as f32 / ADC_SAMPLE_RATE as f32);

        for frame in 0..stereo_frames {
            if (self.cw_envelope - self.cw_target_envelope).abs() > ramp * 0.5 {
                if self.cw_target_envelope > self.cw_envelope {
                    self.cw_envelope = (self.cw_envelope + ramp).min(1.0);
                } else {
                    self.cw_envelope = (self.cw_envelope - ramp).max(0.0);
                }
            } else {
                self.cw_envelope = self.cw_target_envelope;
            }

            let raised_cos =
                0.5 * (1.0 - with_cordic(self.cordic, |c| c.cosf(pi * self.cw_envelope)));
            let carrier = with_cordic(self.cordic, |c| c.cosf(self.nco_phase));
            let output = carrier * raised_cos;

            self.nco_phase += self.nco_phase_step;
            if self.nco_phase > 2.0 * pi {
                self.nco_phase -= 2.0 * pi;
            }

            let dac_val = Self::float_to_dac(output);
            if frame * 2 + 1 < ADC_BUFFER_SIZE {
                dac_out[frame * 2] = dac_val;
                dac_out[frame * 2 + 1] = dac_val;
            }
        }
    }

    fn modulate_am(&mut self, audio: &[f32; DSP_BLOCK_SIZE], dac_out: &mut [u32; ADC_BUFFER_SIZE]) {
        let mut mono = [0.0f32; DSP_BLOCK_SIZE];
        for i in 0..DSP_BLOCK_SIZE {
            mono[i] = 0.5 + 0.5 * audio[i].clamp(-1.0, 1.0);
        }
        self.upsample_mono(&mono, dac_out);
    }

    fn modulate_fm(&mut self, audio: &[f32; DSP_BLOCK_SIZE], dac_out: &mut [u32; ADC_BUFFER_SIZE]) {
        let pi = core::f32::consts::PI;
        let stereo_frames = ADC_BUFFER_SIZE / 2;
        let ratio = stereo_frames / DSP_BLOCK_SIZE;
        let deviation_step = 2.0 * pi * FM_DEVIATION_HZ / ADC_SAMPLE_RATE as f32;

        for i in 0..DSP_BLOCK_SIZE {
            let mod_val = audio[i].clamp(-1.0, 1.0);
            for r in 0..ratio {
                self.fm_mod_phase += self.nco_phase_step + deviation_step * mod_val;
                if self.fm_mod_phase > 2.0 * pi {
                    self.fm_mod_phase -= 2.0 * pi;
                }

                let output = with_cordic(self.cordic, |c| c.cosf(self.fm_mod_phase));
                let dac_val = Self::float_to_dac(output);
                let frame_idx = i * ratio + r;
                if frame_idx * 2 + 1 < ADC_BUFFER_SIZE {
                    dac_out[frame_idx * 2] = dac_val;
                    dac_out[frame_idx * 2 + 1] = dac_val;
                }
            }
        }
    }

    fn upsample_iq(
        &mut self,
        i_buf: &[f32; DSP_BLOCK_SIZE],
        q_buf: &[f32; DSP_BLOCK_SIZE],
        dac_out: &mut [u32; ADC_BUFFER_SIZE],
    ) {
        let pi = core::f32::consts::PI;
        let stereo_frames = ADC_BUFFER_SIZE / 2;
        let ratio = stereo_frames / DSP_BLOCK_SIZE;

        for i in 0..DSP_BLOCK_SIZE {
            for r in 0..ratio {
                let val_i = if r == 0 { i_buf[i] * ratio as f32 } else { 0.0 };
                let val_q = if r == 0 { q_buf[i] * ratio as f32 } else { 0.0 };
                let filt_i = self.interp_fir_i.process_sample(val_i);
                let filt_q = self.interp_fir_q.process_sample(val_q);

                let (sin_val, cos_val) = with_cordic(self.cordic, |c| c.sin_cos(self.nco_phase));
                let output = filt_i * cos_val - filt_q * sin_val;

                self.nco_phase += self.nco_phase_step;
                if self.nco_phase > 2.0 * pi {
                    self.nco_phase -= 2.0 * pi;
                }

                let dac_val = Self::float_to_dac(output);
                let frame_idx = i * ratio + r;
                if frame_idx * 2 + 1 < ADC_BUFFER_SIZE {
                    dac_out[frame_idx * 2] = dac_val;
                    dac_out[frame_idx * 2 + 1] = dac_val;
                }
            }
        }
    }

    fn upsample_mono(
        &mut self,
        mono: &[f32; DSP_BLOCK_SIZE],
        dac_out: &mut [u32; ADC_BUFFER_SIZE],
    ) {
        let pi = core::f32::consts::PI;
        let stereo_frames = ADC_BUFFER_SIZE / 2;
        let ratio = stereo_frames / DSP_BLOCK_SIZE;

        for i in 0..DSP_BLOCK_SIZE {
            for r in 0..ratio {
                let val = if r == 0 { mono[i] * ratio as f32 } else { 0.0 };
                let filtered = self.interp_fir_i.process_sample(val);
                let carrier = with_cordic(self.cordic, |c| c.cosf(self.nco_phase));
                let output = filtered * carrier;

                self.nco_phase += self.nco_phase_step;
                if self.nco_phase > 2.0 * pi {
                    self.nco_phase -= 2.0 * pi;
                }

                let dac_val = Self::float_to_dac(output);
                let frame_idx = i * ratio + r;
                if frame_idx * 2 + 1 < ADC_BUFFER_SIZE {
                    dac_out[frame_idx * 2] = dac_val;
                    dac_out[frame_idx * 2 + 1] = dac_val;
                }
            }
        }
    }

    fn float_to_dac(val: f32) -> u32 {
        ((val * 8_388_607.0).clamp(-8_388_607.0, 8_388_607.0) as i32 & 0x00FF_FFFF) as u32
    }

    fn init_hilbert(&mut self) {
        let mut coeffs = [0.0f32; MAX_FIR_TAPS];
        let taps = 31;
        let half = taps / 2;
        for i in 0..taps {
            let n = i as i32 - half as i32;
            if n % 2 != 0 {
                coeffs[i] = 2.0 / (core::f32::consts::PI * n as f32);
            }
        }
        self.hilbert_fir.load_coefficients(&coeffs, taps);
    }

    fn init_interp_filters(&mut self) {
        let mut coeffs = [0.0f32; MAX_FIR_TAPS];
        SoftwareFir::compute_lowpass_coeffs(
            20000.0,
            ADC_SAMPLE_RATE as f32,
            TX_INTERP_TAPS,
            self.cordic,
            &mut coeffs,
        );
        self.interp_fir_i.load_coefficients(&coeffs, TX_INTERP_TAPS);
        self.interp_fir_q.load_coefficients(&coeffs, TX_INTERP_TAPS);
    }
}
