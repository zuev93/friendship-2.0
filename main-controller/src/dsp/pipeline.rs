use embassy_stm32::peripherals::FMAC;
use embassy_stm32::Peri;

use super::agc::{AnalogAgc, DigitalAgc};
use super::ddc::Ddc;
use super::demod::Demodulator;
use super::fft::FftEngine;
use super::fir::{SoftwareFir, MAX_FIR_TAPS};
use super::nb::NoiseBlanker;
use super::smeter::Smeter;
use super::tx::TxModulator;
use super::types::{AgcPreset, DemodMode, FftResult, FilterPreset, IqBuffer, DSP_BLOCK_SIZE};
use crate::app::cordic_math::CordicMutex;
use crate::app::fmac_fir::{FmacFir, FIR_TAPS};
use crate::consts::{ADC_BUFFER_SIZE, AUDIO_BUFFER_SIZE, DSP_SAMPLE_RATE};

pub struct DspPipeline {
    ddc: Ddc,
    fmac: FmacFir,
    fir_q: SoftwareFir,
    nb: NoiseBlanker,
    agc: DigitalAgc,
    analog_agc: AnalogAgc,
    demod: Demodulator,
    fft: FftEngine,
    smeter: Smeter,
    tx: TxModulator,
    cordic: &'static CordicMutex,
    filter_enabled: bool,
    bw_hz: f32,
    shift_hz: f32,
    fir_coeffs: [f32; FIR_TAPS],
    iq_buf: IqBuffer,
    demod_buf: [f32; DSP_BLOCK_SIZE],
    cw_peak_enabled: bool,
    cw_peak_filter: SoftwareFir,
    cw_pitch_hz: f32,
    cw_peak_bw_hz: f32,
    audio_lpf: SoftwareFir,
    adc_overload: bool,
}

impl DspPipeline {
    pub fn new(cordic: &'static CordicMutex, fmac_peri: Peri<'static, FMAC>) -> Self {
        let mut pipeline = Self {
            ddc: Ddc::new(cordic),
            fmac: FmacFir::new(fmac_peri),
            fir_q: SoftwareFir::new(),
            nb: NoiseBlanker::new(),
            agc: DigitalAgc::new(cordic),
            analog_agc: AnalogAgc::new(),
            demod: Demodulator::new(cordic),
            fft: FftEngine::new(cordic),
            smeter: Smeter::new(),
            tx: TxModulator::new(cordic),
            cordic,
            filter_enabled: false,
            bw_hz: 2700.0,
            shift_hz: 0.0,
            fir_coeffs: [0.0; FIR_TAPS],
            iq_buf: IqBuffer::zero(),
            demod_buf: [0.0; DSP_BLOCK_SIZE],
            cw_peak_enabled: false,
            cw_peak_filter: SoftwareFir::new(),
            cw_pitch_hz: 700.0,
            cw_peak_bw_hz: 200.0,
            audio_lpf: SoftwareFir::new(),
            adc_overload: false,
        };
        pipeline.fft.init();
        pipeline.set_nco_frequency(super::types::NCO_CENTER_HZ);
        pipeline.set_smeter_calibration(0.0);
        pipeline.init_audio_lpf();
        pipeline
    }

    pub fn process_rx(&mut self, adc_buffer: &[u32; ADC_BUFFER_SIZE]) -> [u16; AUDIO_BUFFER_SIZE] {
        self.detect_adc_overload(adc_buffer);

        self.ddc.process(adc_buffer, &mut self.iq_buf);

        self.nb.process(&mut self.iq_buf);

        if self.filter_enabled {
            self.apply_selectivity_filter();
        }

        self.agc.process(&mut self.iq_buf);

        self.demod.process(&self.iq_buf, &mut self.demod_buf);

        if self.cw_peak_enabled {
            for i in 0..DSP_BLOCK_SIZE {
                self.demod_buf[i] = self.cw_peak_filter.process_sample(self.demod_buf[i]);
            }
        }

        for i in 0..DSP_BLOCK_SIZE {
            self.demod_buf[i] = self.audio_lpf.process_sample(self.demod_buf[i]);
        }

        let analog_gain_db = self.analog_agc.gain_db();
        self.smeter
            .update(self.agc.current_level_db(), analog_gain_db);

        self.float_to_u16()
    }

    pub fn process_rx_with_fft(
        &mut self,
        adc_buffer: &[u32; ADC_BUFFER_SIZE],
    ) -> ([u16; AUDIO_BUFFER_SIZE], FftResult) {
        let fft_result = self.fft.process(adc_buffer);
        let audio = self.process_rx(adc_buffer);
        (audio, fft_result)
    }

    pub fn process_adc_peak(&mut self, peak_dbfs: f32) -> u16 {
        self.analog_agc.process_adc_peak(peak_dbfs)
    }

    pub fn smeter_dbm(&self) -> f32 {
        self.smeter.dbm()
    }

    pub fn smeter_s_units(&self) -> f32 {
        self.smeter.s_units()
    }

    pub fn smeter_s_string(&self) -> (u8, i8) {
        self.smeter.s_string()
    }

    pub fn agc_current_gain(&self) -> f32 {
        self.agc.current_gain()
    }

    pub fn process_tx(
        &mut self,
        audio_in: &[u16; AUDIO_BUFFER_SIZE],
        dac_out: &mut [u32; ADC_BUFFER_SIZE],
    ) {
        let mut float_buf = [0.0f32; DSP_BLOCK_SIZE];
        for i in 0..DSP_BLOCK_SIZE {
            float_buf[i] = audio_in[i] as f32 / 32768.0 - 1.0;
        }
        self.tx.process(&float_buf, dac_out);
    }

    pub fn set_demod_mode(&mut self, mode: DemodMode) {
        self.demod.set_mode(mode);
        self.tx.set_mode(mode);
        let preset = FilterPreset::for_mode(mode);
        self.apply_filter_preset(&preset);
    }

    pub fn apply_filter_preset(&mut self, preset: &FilterPreset) {
        self.bw_hz = preset.bw_hz;
        self.shift_hz = preset.shift_hz;
        self.recompute_filter_with_taps(preset.taps);
    }

    pub fn set_filter_enabled(&mut self, enabled: bool) {
        self.filter_enabled = enabled;
        if enabled {
            self.recompute_filter();
        } else {
            self.fmac.stop();
            self.fir_q.reset();
        }
    }

    pub fn set_bandwidth(&mut self, bw_hz: f32) {
        self.bw_hz = bw_hz;
        if self.filter_enabled {
            self.recompute_filter();
        }
    }

    pub fn set_shift(&mut self, shift_hz: f32) {
        self.shift_hz = shift_hz;
        if self.filter_enabled {
            self.recompute_filter();
        }
    }

    pub fn set_nb_enabled(&mut self, on: bool) {
        self.nb.set_enabled(on);
    }

    pub fn set_nb_threshold(&mut self, level: u8) {
        self.nb.set_threshold(level);
    }

    pub fn set_agc_preset(&mut self, preset: AgcPreset) {
        self.agc.set_preset(preset);
    }

    pub fn set_agc_manual_gain(&mut self, gain_db: f32) {
        self.agc.set_manual_gain_db(gain_db);
    }

    pub fn set_nco_frequency(&mut self, freq_hz: u32) {
        self.ddc.set_frequency(freq_hz);
    }

    pub fn set_rit_offset(&mut self, offset_hz: i32) {
        self.ddc.set_rit_offset(offset_hz);
    }

    pub fn set_cw_key(&mut self, down: bool) {
        self.tx.set_cw_key(down);
    }

    pub fn set_cw_pitch(&mut self, pitch_hz: u16) {
        self.cw_pitch_hz = pitch_hz as f32;
        self.tx.set_cw_pitch(pitch_hz);
        if self.cw_peak_enabled {
            self.recompute_cw_peak_filter();
        }
    }

    pub fn set_smeter_calibration(&mut self, offset: f32) {
        self.smeter.set_calibration(offset);
    }

    pub fn set_cw_peak_enabled(&mut self, enabled: bool) {
        self.cw_peak_enabled = enabled;
        if enabled {
            self.recompute_cw_peak_filter();
        } else {
            self.cw_peak_filter.reset();
        }
    }

    pub fn set_cw_peak_bw(&mut self, bw_hz: f32) {
        self.cw_peak_bw_hz = bw_hz;
        if self.cw_peak_enabled {
            self.recompute_cw_peak_filter();
        }
    }

    pub fn adc_overload(&self) -> bool {
        self.adc_overload
    }

    fn recompute_cw_peak_filter(&mut self) {
        let mut coeffs = [0.0f32; MAX_FIR_TAPS];
        let taps = 63;
        SoftwareFir::compute_bandpass_coeffs(
            self.cw_peak_bw_hz,
            self.cw_pitch_hz,
            DSP_SAMPLE_RATE as f32,
            taps,
            self.cordic,
            &mut coeffs,
        );
        self.cw_peak_filter.load_coefficients(&coeffs, taps);
    }

    fn init_audio_lpf(&mut self) {
        let mut coeffs = [0.0f32; MAX_FIR_TAPS];
        let taps = 31;
        SoftwareFir::compute_lowpass_coeffs(
            3400.0,
            DSP_SAMPLE_RATE as f32,
            taps,
            self.cordic,
            &mut coeffs,
        );
        self.audio_lpf.load_coefficients(&coeffs, taps);
    }

    fn recompute_filter(&mut self) {
        self.recompute_filter_with_taps(FIR_TAPS);
    }

    fn recompute_filter_with_taps(&mut self, taps: usize) {
        let num_taps = taps.min(FIR_TAPS);
        let mut coeffs_f32 = [0.0f32; MAX_FIR_TAPS];
        SoftwareFir::compute_bandpass_coeffs(
            self.bw_hz,
            self.shift_hz,
            DSP_SAMPLE_RATE as f32,
            num_taps,
            self.cordic,
            &mut coeffs_f32,
        );

        for i in 0..num_taps {
            self.fir_coeffs[i] = coeffs_f32[i];
        }
        for i in num_taps..FIR_TAPS {
            self.fir_coeffs[i] = 0.0;
        }

        self.fmac.stop();
        self.fmac.load_coefficients(&self.fir_coeffs);
        self.fmac.start_fir();

        self.fir_q.load_coefficients(&coeffs_f32, num_taps);
    }

    fn detect_adc_overload(&mut self, adc_buffer: &[u32; ADC_BUFFER_SIZE]) {
        const OVERLOAD_THRESHOLD: i32 = 8_300_000;
        let mono_samples = ADC_BUFFER_SIZE / 2;
        let mut overloaded = false;
        for frame in 0..mono_samples {
            let raw = adc_buffer[frame * 2];
            let signed_24 = ((raw << 8) as i32) >> 8;
            let abs = if signed_24 < 0 { -signed_24 } else { signed_24 };
            if abs > OVERLOAD_THRESHOLD {
                overloaded = true;
                break;
            }
        }
        self.adc_overload = overloaded;
    }

    fn apply_selectivity_filter(&mut self) {
        for i in 0..DSP_BLOCK_SIZE {
            let i_sample = self.iq_buf.i[i];
            let i_q15 = (i_sample / 256.0).clamp(-32768.0, 32767.0) as i16;
            let filtered_q15 = self.fmac.process_sample(i_q15);
            self.iq_buf.i[i] = filtered_q15 as f32 * 256.0;

            self.iq_buf.q[i] = self.fir_q.process_sample(self.iq_buf.q[i]);
        }
    }

    fn float_to_u16(&self) -> [u16; AUDIO_BUFFER_SIZE] {
        let mut out = [0u16; AUDIO_BUFFER_SIZE];
        let mut peak = 0.0f32;
        for i in 0..DSP_BLOCK_SIZE {
            let abs = if self.demod_buf[i] < 0.0 {
                -self.demod_buf[i]
            } else {
                self.demod_buf[i]
            };
            if abs > peak {
                peak = abs;
            }
        }

        let scale = if peak > 1.0 { 1.0 / peak } else { 1.0 };

        for i in 0..AUDIO_BUFFER_SIZE {
            let normalized = self.demod_buf[i] * scale;
            out[i] = ((normalized + 1.0) * 32768.0).clamp(0.0, 65535.0) as u16;
        }
        out
    }
}
