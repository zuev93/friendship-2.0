use super::fir::{compute_lowpass_coeffs, SoftwareFir, MAX_FIR_TAPS};
use super::types::{DemodMode, DSP_BLOCK_SIZE};
use crate::app::cordic_math::{with_cordic, CordicMutex};
use crate::consts::{ADC_BUFFER_SIZE, ADC_SAMPLE_RATE, DSP_SAMPLE_RATE};

const CW_RISE_MS: f32 = 5.0;
const TX_INTERP_TAPS: usize = 63;

pub struct TxModulator {
    mode: DemodMode,
    hilbert_fir: SoftwareFir,
    delay_line: [f32; 32],
    delay_idx: usize,
    cw_phase: u32,
    cw_phase_step: u32,
    cw_envelope: f32,
    cw_envelope_step: f32,
    cw_key_down: bool,
    interp_fir: SoftwareFir,
    cordic: &'static CordicMutex,
}

impl TxModulator {
    pub fn new(cordic: &'static CordicMutex) -> Self {
        let mut tx = Self {
            mode: DemodMode::Usb,
            hilbert_fir: SoftwareFir::new(),
            delay_line: [0.0; 32],
            delay_idx: 0,
            cw_phase: 0,
            cw_phase_step: 0,
            cw_envelope: 0.0,
            cw_envelope_step: 1.0 / (CW_RISE_MS * DSP_SAMPLE_RATE as f32 / 1000.0),
            cw_key_down: false,
            interp_fir: SoftwareFir::new(),
            cordic,
        };
        tx.init_hilbert();
        tx.init_interp_filter();
        tx
    }

    pub fn set_mode(&mut self, mode: DemodMode) {
        self.mode = mode;
    }

    pub fn set_cw_key(&mut self, down: bool) {
        self.cw_key_down = down;
    }

    pub fn process(
        &mut self,
        audio_in: &[f32; DSP_BLOCK_SIZE],
        dac_out: &mut [u32; ADC_BUFFER_SIZE],
    ) {
        match self.mode {
            DemodMode::Usb => self.modulate_ssb(audio_in, dac_out, false),
            DemodMode::Lsb => self.modulate_ssb(audio_in, dac_out, true),
            DemodMode::Cw => self.modulate_cw(dac_out),
            DemodMode::Am => self.modulate_am(audio_in, dac_out),
            DemodMode::Fm | DemodMode::Sam => self.modulate_ssb(audio_in, dac_out, false),
        }
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
            self.delay_line[self.delay_idx] = audio[idx];
            self.delay_idx = (self.delay_idx + 1) % 32;
            let delay_read = (self.delay_idx + 32 - 16) % 32;
            i_buf[idx] = self.delay_line[delay_read];
            q_buf[idx] = self.hilbert_fir.process_sample(audio[idx]);
        }

        if invert_q {
            for s in q_buf.iter_mut() {
                *s = -*s;
            }
        }

        self.upsample_iq(&i_buf, &q_buf, dac_out);
    }

    fn modulate_cw(&mut self, dac_out: &mut [u32; ADC_BUFFER_SIZE]) {
        let stereo_frames = ADC_BUFFER_SIZE / 2;

        for frame in 0..stereo_frames {
            if self.cw_key_down {
                self.cw_envelope = (self.cw_envelope + self.cw_envelope_step).min(1.0);
            } else {
                self.cw_envelope = (self.cw_envelope - self.cw_envelope_step).max(0.0);
            }

            let phase_rad = phase_to_radians(self.cw_phase);
            let (_, cos_val) = with_cordic(self.cordic, |c| c.sin_cos(phase_rad));
            self.cw_phase = self.cw_phase.wrapping_add(self.cw_phase_step);

            let sample = cos_val * self.cw_envelope;
            let dac_val = ((sample * 8_388_607.0) as i32 & 0x00FF_FFFF) as u32;
            dac_out[frame * 2] = dac_val;
            dac_out[frame * 2 + 1] = dac_val;
        }
    }

    fn modulate_am(&mut self, audio: &[f32; DSP_BLOCK_SIZE], dac_out: &mut [u32; ADC_BUFFER_SIZE]) {
        let mut carrier = [0.0f32; DSP_BLOCK_SIZE];
        for idx in 0..DSP_BLOCK_SIZE {
            carrier[idx] = 0.5 + 0.5 * audio[idx].clamp(-1.0, 1.0);
        }
        self.upsample_mono(&carrier, dac_out);
    }

    fn upsample_iq(
        &mut self,
        i_buf: &[f32; DSP_BLOCK_SIZE],
        _q_buf: &[f32; DSP_BLOCK_SIZE],
        dac_out: &mut [u32; ADC_BUFFER_SIZE],
    ) {
        self.upsample_mono(i_buf, dac_out);
    }

    fn upsample_mono(
        &mut self,
        mono: &[f32; DSP_BLOCK_SIZE],
        dac_out: &mut [u32; ADC_BUFFER_SIZE],
    ) {
        let stereo_frames = ADC_BUFFER_SIZE / 2;
        let ratio = stereo_frames / DSP_BLOCK_SIZE;

        for i in 0..DSP_BLOCK_SIZE {
            for r in 0..ratio {
                let val = if r == 0 { mono[i] } else { 0.0 };
                let filtered = self.interp_fir.process_sample(val * ratio as f32);
                let dac_val = ((filtered * 8_388_607.0).clamp(-8_388_607.0, 8_388_607.0) as i32
                    & 0x00FF_FFFF) as u32;
                let frame_idx = i * ratio + r;
                if frame_idx * 2 + 1 < ADC_BUFFER_SIZE {
                    dac_out[frame_idx * 2] = dac_val;
                    dac_out[frame_idx * 2 + 1] = dac_val;
                }
            }
        }
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

    fn init_interp_filter(&mut self) {
        let mut coeffs = [0.0f32; MAX_FIR_TAPS];
        compute_lowpass_coeffs(
            20000.0,
            ADC_SAMPLE_RATE as f32,
            TX_INTERP_TAPS,
            self.cordic,
            &mut coeffs,
        );
        self.interp_fir.load_coefficients(&coeffs, TX_INTERP_TAPS);
    }
}

fn phase_to_radians(phase: u32) -> f32 {
    let signed = phase as i32;
    signed as f32 * (core::f32::consts::PI / 2_147_483_648.0)
}
