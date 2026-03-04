use super::types::{IqBuffer, DSP_BLOCK_SIZE, NCO_CENTER_HZ};
use crate::app::cordic_math::{with_cordic, CordicMutex};
use crate::consts::{ADC_BUFFER_SIZE, DSP_SAMPLE_RATE};

const CIC_GAIN: f32 = 64.0;
const DECIMATION: usize = 4;
const ADC_MONO_SAMPLES: usize = ADC_BUFFER_SIZE / 2;

struct Cic3State {
    integ1: i32,
    integ2: i32,
    integ3: i32,
    comb1_delay: i32,
    comb2_delay: i32,
    comb3_delay: i32,
}

impl Cic3State {
    const fn new() -> Self {
        Self {
            integ1: 0,
            integ2: 0,
            integ3: 0,
            comb1_delay: 0,
            comb2_delay: 0,
            comb3_delay: 0,
        }
    }

    fn integrate(&mut self, sample: i32) {
        self.integ1 = self.integ1.wrapping_add(sample);
        self.integ2 = self.integ2.wrapping_add(self.integ1);
        self.integ3 = self.integ3.wrapping_add(self.integ2);
    }

    fn comb(&mut self) -> i32 {
        let c1 = self.integ3.wrapping_sub(self.comb1_delay);
        self.comb1_delay = self.integ3;
        let c2 = c1.wrapping_sub(self.comb2_delay);
        self.comb2_delay = c1;
        let c3 = c2.wrapping_sub(self.comb3_delay);
        self.comb3_delay = c2;
        c3
    }
}

pub struct Ddc {
    nco_phase: u32,
    nco_phase_step: u32,
    cic_i: Cic3State,
    cic_q: Cic3State,
    decim_counter: usize,
    cordic: &'static CordicMutex,
}

impl Ddc {
    pub fn new(cordic: &'static CordicMutex) -> Self {
        let phase_step = compute_phase_step(NCO_CENTER_HZ, crate::consts::ADC_SAMPLE_RATE);
        Self {
            nco_phase: 0,
            nco_phase_step: phase_step,
            cic_i: Cic3State::new(),
            cic_q: Cic3State::new(),
            decim_counter: 0,
            cordic,
        }
    }

    pub fn set_frequency(&mut self, freq_hz: u32) {
        self.nco_phase_step = compute_phase_step(freq_hz, crate::consts::ADC_SAMPLE_RATE);
    }

    pub fn set_rit_offset(&mut self, offset_hz: i16) {
        let freq = (NCO_CENTER_HZ as i32 + offset_hz as i32).max(0) as u32;
        self.set_frequency(freq);
    }

    pub fn process(&mut self, adc_buffer: &[u32; ADC_BUFFER_SIZE], output: &mut IqBuffer) {
        let mut out_idx = 0;

        for frame in 0..ADC_MONO_SAMPLES {
            let raw = adc_buffer[frame * 2];
            let signed_24 = ((raw << 8) as i32) >> 8;

            let phase_rad = phase_to_radians(self.nco_phase);
            let (sin_val, cos_val) = with_cordic(self.cordic, |c| c.sin_cos(phase_rad));
            self.nco_phase = self.nco_phase.wrapping_add(self.nco_phase_step);

            let input_f = signed_24 as f32;
            let i_sample = (input_f * cos_val) as i32;
            let q_sample = (input_f * (-sin_val)) as i32;

            self.cic_i.integrate(i_sample);
            self.cic_q.integrate(q_sample);

            self.decim_counter += 1;
            if self.decim_counter >= DECIMATION {
                self.decim_counter = 0;
                if out_idx < DSP_BLOCK_SIZE {
                    let ci = self.cic_i.comb();
                    let cq = self.cic_q.comb();
                    output.i[out_idx] = ci as f32 / CIC_GAIN;
                    output.q[out_idx] = cq as f32 / CIC_GAIN;
                    out_idx += 1;
                }
            }
        }
    }
}

fn compute_phase_step(freq_hz: u32, sample_rate: u32) -> u32 {
    ((freq_hz as u64 * (1u64 << 32)) / sample_rate as u64) as u32
}

fn phase_to_radians(phase: u32) -> f32 {
    let signed = phase as i32;
    signed as f32 * (core::f32::consts::PI / 2_147_483_648.0)
}
