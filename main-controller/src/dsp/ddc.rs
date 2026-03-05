use super::types::{IqBuffer, DSP_BLOCK_SIZE, NCO_CENTER_HZ};
use crate::consts::ADC_BUFFER_SIZE;
use crate::cordic_math::{with_cordic, CordicMutex};

const CIC_GAIN: f32 = 64.0;
const DECIMATION: usize = 4;
const ADC_MONO_SAMPLES: usize = ADC_BUFFER_SIZE / 2;

const CIC_COMP_TAPS: usize = 15;
const CIC_COMP_COEFFS: [f32; CIC_COMP_TAPS] = [
    -0.0018, 0.0042, -0.0103, 0.0223, -0.0455, 0.0935, -0.2108, 0.6968, -0.2108, 0.0935, -0.0455,
    0.0223, -0.0103, 0.0042, -0.0018,
];

struct CicCompFir {
    delay: [f32; CIC_COMP_TAPS],
    idx: usize,
}

impl CicCompFir {
    const fn new() -> Self {
        Self {
            delay: [0.0; CIC_COMP_TAPS],
            idx: 0,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        self.delay[self.idx] = input;
        let mut acc = 0.0f32;
        let mut di = self.idx;
        let mut ci = 0;
        while ci < CIC_COMP_TAPS {
            acc += CIC_COMP_COEFFS[ci] * self.delay[di];
            if di == 0 {
                di = CIC_COMP_TAPS - 1;
            } else {
                di -= 1;
            }
            ci += 1;
        }
        self.idx += 1;
        if self.idx >= CIC_COMP_TAPS {
            self.idx = 0;
        }
        acc
    }
}

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
    base_phase_step: u32,
    rit_phase_step: i32,
    cic_i: Cic3State,
    cic_q: Cic3State,
    comp_i: CicCompFir,
    comp_q: CicCompFir,
    decim_counter: usize,
    cordic: &'static CordicMutex,
}

impl Ddc {
    pub fn new(cordic: &'static CordicMutex) -> Self {
        let phase_step = Self::compute_phase_step(NCO_CENTER_HZ, crate::consts::ADC_SAMPLE_RATE);
        Self {
            nco_phase: 0,
            nco_phase_step: phase_step,
            base_phase_step: phase_step,
            rit_phase_step: 0,
            cic_i: Cic3State::new(),
            cic_q: Cic3State::new(),
            comp_i: CicCompFir::new(),
            comp_q: CicCompFir::new(),
            decim_counter: 0,
            cordic,
        }
    }

    pub fn set_frequency(&mut self, freq_hz: u32) {
        self.base_phase_step = Self::compute_phase_step(freq_hz, crate::consts::ADC_SAMPLE_RATE);
        self.update_phase_step();
    }

    pub fn set_rit_offset(&mut self, offset_hz: i32) {
        let abs_hz = if offset_hz < 0 {
            (-offset_hz) as u32
        } else {
            offset_hz as u32
        };
        let abs_step = Self::compute_phase_step(abs_hz, crate::consts::ADC_SAMPLE_RATE);
        self.rit_phase_step = if offset_hz < 0 {
            -(abs_step as i32)
        } else {
            abs_step as i32
        };
        self.update_phase_step();
    }

    fn update_phase_step(&mut self) {
        self.nco_phase_step =
            (self.base_phase_step as i32).wrapping_add(self.rit_phase_step) as u32;
    }

    pub fn process(&mut self, adc_buffer: &[u32; ADC_BUFFER_SIZE], output: &mut IqBuffer) {
        let mut out_idx = 0;

        for frame in 0..ADC_MONO_SAMPLES {
            let raw = adc_buffer[frame * 2];
            let signed_24 = ((raw << 8) as i32) >> 8;

            let phase_rad = Self::phase_to_radians(self.nco_phase);
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
                    let i_raw = ci as f32 / CIC_GAIN;
                    let q_raw = cq as f32 / CIC_GAIN;
                    output.i[out_idx] = self.comp_i.process(i_raw);
                    output.q[out_idx] = self.comp_q.process(q_raw);
                    out_idx += 1;
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
}
