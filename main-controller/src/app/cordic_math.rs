use embassy_stm32::cordic::{
    self,
    utils::{f64_to_q1_31, q1_31_to_f64},
    Config, Function, Precision, Scale,
};
use embassy_stm32::peripherals::CORDIC;
use embassy_stm32::Peri;

const PI: f64 = core::f64::consts::PI;

const SQRT_2: f32 = 1.4142135;

const DB_TO_AMP: [f32; 25] = [
    0.25119, 0.26607, 0.28184, 0.29854, 0.31623, 0.33497, 0.35481, 0.37584, 0.39811, 0.42170,
    0.44668, 0.47315, 0.50119, 0.53088, 0.56234, 0.59566, 0.63096, 0.66834, 0.70795, 0.74989,
    0.79433, 0.84140, 0.89125, 0.94406, 1.00000,
];

fn wrap_to_q1(normalized: f64) -> f64 {
    if normalized >= -1.0 && normalized < 1.0 {
        return normalized;
    }
    let mut v = normalized;
    let periods = (v / 2.0) as i64;
    v -= periods as f64 * 2.0;
    if v >= 1.0 {
        v -= 2.0;
    } else if v < -1.0 {
        v += 2.0;
    }
    v
}

pub struct CordicMath {
    cordic: cordic::Cordic<'static, CORDIC>,
    current_func: Function,
}

impl CordicMath {
    pub fn new(peri: Peri<'static, CORDIC>) -> Self {
        let config = Config::new(Function::Cos, Precision::Iters24, Scale::Arg1Res1).unwrap();
        Self {
            cordic: cordic::Cordic::new(peri, config),
            current_func: Function::Cos,
        }
    }

    fn configure(&mut self, func: Function) {
        if self.current_func as u8 != func as u8 {
            let scale = match func {
                Function::Sqrt => Scale::Arg1Res1,
                _ => Scale::Arg1Res1,
            };
            let config = Config::new(func, Precision::Iters24, scale).unwrap();
            self.cordic.set_config(config);
            self.current_func = func;
        }
    }

    pub fn sinf(&mut self, radians: f32) -> f32 {
        self.configure(Function::Sin);
        let clamped = wrap_to_q1(radians as f64 / PI);
        let q = f64_to_q1_31(clamped).unwrap();
        let mut res = [0u32; 1];
        self.cordic
            .blocking_calc_32bit(&[q], &mut res, true, true)
            .unwrap();
        q1_31_to_f64(res[0]) as f32
    }

    pub fn cosf(&mut self, radians: f32) -> f32 {
        self.configure(Function::Cos);
        let clamped = wrap_to_q1(radians as f64 / PI);
        let q = f64_to_q1_31(clamped).unwrap();
        let mut res = [0u32; 1];
        self.cordic
            .blocking_calc_32bit(&[q], &mut res, true, true)
            .unwrap();
        q1_31_to_f64(res[0]) as f32
    }

    pub fn sin_cos(&mut self, radians: f32) -> (f32, f32) {
        self.configure(Function::Cos);
        let clamped = wrap_to_q1(radians as f64 / PI);
        let q = f64_to_q1_31(clamped).unwrap();
        let mut res = [0u32; 2];
        self.cordic
            .blocking_calc_32bit(&[q], &mut res, true, false)
            .unwrap();
        let cos_val = q1_31_to_f64(res[0]) as f32;
        let sin_val = q1_31_to_f64(res[1]) as f32;
        (sin_val, cos_val)
    }

    pub fn db_to_amplitude(db: i8) -> f32 {
        let idx = (db as i16 + 12) as usize;
        if idx >= DB_TO_AMP.len() {
            DB_TO_AMP[DB_TO_AMP.len() - 1]
        } else {
            DB_TO_AMP[idx]
        }
    }

    pub fn fast_exp(x: f32) -> f32 {
        1.0 + x + x * x * 0.5
    }

    pub fn fast_sqrt(x: f32) -> f32 {
        if x <= 0.0 {
            return 0.0;
        }
        let mut guess = x * 0.5 + 0.5;
        guess = 0.5 * (guess + x / guess);
        guess = 0.5 * (guess + x / guess);
        guess
    }

    pub fn sqrt_2() -> f32 {
        SQRT_2
    }
}
