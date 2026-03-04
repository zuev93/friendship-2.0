use super::types::{IqBuffer, DSP_BLOCK_SIZE};

const NB_WINDOW_SAMPLES: usize = 48;
const NB_AVG_ALPHA: f32 = 0.001;

pub struct NoiseBlanker {
    enabled: bool,
    threshold: f32,
    avg_level: f32,
}

impl NoiseBlanker {
    pub const fn new() -> Self {
        Self {
            enabled: false,
            threshold: 4.0,
            avg_level: 0.001,
        }
    }

    pub fn set_enabled(&mut self, on: bool) {
        self.enabled = on;
        if !on {
            self.avg_level = 0.001;
        }
    }

    pub fn set_threshold(&mut self, level: u8) {
        self.threshold = 2.0 + level as f32 * 0.5;
    }

    pub fn process(&mut self, iq: &mut IqBuffer) {
        if !self.enabled {
            return;
        }

        for idx in 0..DSP_BLOCK_SIZE {
            let mag = iq.i[idx] * iq.i[idx] + iq.q[idx] * iq.q[idx];

            self.avg_level += NB_AVG_ALPHA * (mag - self.avg_level);

            if mag > self.avg_level * self.threshold * self.threshold {
                let start = if idx >= NB_WINDOW_SAMPLES / 2 {
                    idx - NB_WINDOW_SAMPLES / 2
                } else {
                    0
                };
                let end = (idx + NB_WINDOW_SAMPLES / 2).min(DSP_BLOCK_SIZE);

                let before_i = if start > 0 { iq.i[start - 1] } else { 0.0 };
                let before_q = if start > 0 { iq.q[start - 1] } else { 0.0 };
                let after_i = if end < DSP_BLOCK_SIZE { iq.i[end] } else { 0.0 };
                let after_q = if end < DSP_BLOCK_SIZE { iq.q[end] } else { 0.0 };

                let len = (end - start) as f32;
                for k in start..end {
                    let t = (k - start) as f32 / len;
                    iq.i[k] = before_i * (1.0 - t) + after_i * t;
                    iq.q[k] = before_q * (1.0 - t) + after_q * t;
                }
            }
        }
    }
}
