pub const FIR_TAPS: usize = 127;

fn f32_to_q15(x: f32) -> i16 {
    (x * 32767.0).clamp(-32768.0, 32767.0) as i16
}

pub struct FmacFir {
    coeffs_q15: [i16; FIR_TAPS],
    history: [i16; FIR_TAPS],
    pos: usize,
}

impl FmacFir {
    pub fn new() -> Self {
        Self {
            coeffs_q15: [0; FIR_TAPS],
            history: [0; FIR_TAPS],
            pos: 0,
        }
    }

    pub fn load_coefficients(&mut self, coeffs: &[f32; FIR_TAPS]) {
        for i in 0..FIR_TAPS {
            self.coeffs_q15[i] = f32_to_q15(coeffs[i]);
        }
        self.history = [0; FIR_TAPS];
        self.pos = 0;
    }

    pub fn start_fir(&mut self) {}

    pub fn stop(&mut self) {
        self.history = [0; FIR_TAPS];
        self.pos = 0;
    }

    pub fn process_sample(&mut self, sample: i16) -> i16 {
        self.history[self.pos] = sample;
        let mut acc: i32 = 0;
        let mut idx = self.pos;
        for i in 0..FIR_TAPS {
            acc += self.history[idx] as i32 * self.coeffs_q15[i] as i32;
            if idx == 0 {
                idx = FIR_TAPS - 1;
            } else {
                idx -= 1;
            }
        }
        self.pos = (self.pos + 1) % FIR_TAPS;
        (acc >> 15).clamp(-32768, 32767) as i16
    }
}
