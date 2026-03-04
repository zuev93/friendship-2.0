use core::cell::RefCell;
use embassy_stm32::cordic::{
    self,
    utils::{f64_to_q1_31, q1_31_to_f64},
    Config, Function, Precision, Scale,
};
use embassy_stm32::peripherals::CORDIC;
use embassy_stm32::Peri;
use embassy_sync::blocking_mutex::{raw::CriticalSectionRawMutex, Mutex};
use static_cell::StaticCell;

const PI: f64 = core::f64::consts::PI;
const LN2: f32 = 0.693_147_2;
const LN10: f32 = 2.302_585;
const SQRT_2: f32 = 1.414_213_5;

pub type CordicMutex = Mutex<CriticalSectionRawMutex, RefCell<CordicMath>>;

static CORDIC_INSTANCE: StaticCell<CordicMutex> = StaticCell::new();

pub fn init_global(peri: Peri<'static, CORDIC>) -> &'static CordicMutex {
    CORDIC_INSTANCE.init(Mutex::new(RefCell::new(CordicMath::new(peri))))
}

pub fn with_cordic<R>(mutex: &'static CordicMutex, f: impl FnOnce(&mut CordicMath) -> R) -> R {
    mutex.lock(|cell| f(&mut cell.borrow_mut()))
}

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
    current_scale: Scale,
}

impl CordicMath {
    pub fn new(peri: Peri<'static, CORDIC>) -> Self {
        let config = Config::new(Function::Cos, Precision::Iters24, Scale::Arg1Res1).unwrap();
        Self {
            cordic: cordic::Cordic::new(peri, config),
            current_func: Function::Cos,
            current_scale: Scale::Arg1Res1,
        }
    }

    fn configure(&mut self, func: Function, scale: Scale) {
        if self.current_func as u8 != func as u8 || self.current_scale != scale {
            let config = Config::new(func, Precision::Iters24, scale).unwrap();
            self.cordic.set_config(config);
            self.current_func = func;
            self.current_scale = scale;
        }
    }

    pub fn sinf(&mut self, radians: f32) -> f32 {
        self.configure(Function::Sin, Scale::Arg1Res1);
        let q = f64_to_q1_31(wrap_to_q1(radians as f64 / PI)).unwrap();
        let mut res = [0u32; 1];
        self.cordic
            .blocking_calc_32bit(&[q], &mut res, true, true)
            .unwrap();
        q1_31_to_f64(res[0]) as f32
    }

    pub fn cosf(&mut self, radians: f32) -> f32 {
        self.configure(Function::Cos, Scale::Arg1Res1);
        let q = f64_to_q1_31(wrap_to_q1(radians as f64 / PI)).unwrap();
        let mut res = [0u32; 1];
        self.cordic
            .blocking_calc_32bit(&[q], &mut res, true, true)
            .unwrap();
        q1_31_to_f64(res[0]) as f32
    }

    pub fn sin_cos(&mut self, radians: f32) -> (f32, f32) {
        self.configure(Function::Cos, Scale::Arg1Res1);
        let q = f64_to_q1_31(wrap_to_q1(radians as f64 / PI)).unwrap();
        let mut res = [0u32; 2];
        self.cordic
            .blocking_calc_32bit(&[q], &mut res, true, false)
            .unwrap();
        let cos_val = q1_31_to_f64(res[0]) as f32;
        let sin_val = q1_31_to_f64(res[1]) as f32;
        (sin_val, cos_val)
    }

    pub fn sqrtf(&mut self, x: f32) -> f32 {
        if x <= 0.0 {
            return 0.0;
        }
        self.configure(Function::Sqrt, Scale::Arg1Res1);
        let normalized = (x as f64).clamp(0.027, 0.75);
        let q = f64_to_q1_31(normalized).unwrap();
        let mut res = [0u32; 1];
        self.cordic
            .blocking_calc_32bit(&[q], &mut res, true, true)
            .unwrap();
        q1_31_to_f64(res[0]) as f32
    }

    fn coshf(&mut self, x: f32) -> f32 {
        self.configure(Function::Cosh, Scale::Arg1o2Res2);
        let q = f64_to_q1_31((x as f64).clamp(-0.559, 0.559)).unwrap();
        let mut res = [0u32; 1];
        self.cordic
            .blocking_calc_32bit(&[q], &mut res, true, true)
            .unwrap();
        q1_31_to_f64(res[0]) as f32 * 2.0
    }

    fn sinhf(&mut self, x: f32) -> f32 {
        self.configure(Function::Sinh, Scale::Arg1o2Res2);
        let q = f64_to_q1_31((x as f64).clamp(-0.559, 0.559)).unwrap();
        let mut res = [0u32; 1];
        self.cordic
            .blocking_calc_32bit(&[q], &mut res, true, true)
            .unwrap();
        q1_31_to_f64(res[0]) as f32 * 2.0
    }

    pub fn lnf(&mut self, x: f32) -> f32 {
        if x <= 0.0 {
            return f32::NEG_INFINITY;
        }
        let bits = x.to_bits();
        let exponent = ((bits >> 23) & 0xFF) as i32 - 127;
        let mantissa_bits = (bits & 0x007F_FFFF) | 0x3F80_0000;
        let m = f32::from_bits(mantissa_bits);
        let arg = (m as f64) - 1.0;
        self.configure(Function::Ln, Scale::Arg1Res1);
        let q = f64_to_q1_31(arg.clamp(-0.9999, 0.9999)).unwrap();
        let mut res = [0u32; 1];
        self.cordic
            .blocking_calc_32bit(&[q], &mut res, true, true)
            .unwrap();
        let ln_m = q1_31_to_f64(res[0]) as f32;
        ln_m + exponent as f32 * LN2
    }

    pub fn expf(&mut self, x: f32) -> f32 {
        if x > 0.0 {
            return 1.0 / self.expf(-x);
        }
        if x < -20.0 {
            return 0.0;
        }
        let mut k = 0u32;
        let mut r = x;
        while r < -0.5 {
            r += LN2;
            k += 1;
        }
        let exp_r = self.coshf(r) + self.sinhf(r);
        let scale = if k < 31 {
            1.0f32 / ((1u32 << k) as f32)
        } else {
            0.0
        };
        exp_r * scale
    }

    pub fn pow10f(&mut self, x: f32) -> f32 {
        self.expf(x * LN10)
    }

    pub fn db_to_amplitude(&mut self, db: f32) -> f32 {
        self.pow10f(db / 20.0)
    }

    pub fn sqrt_2() -> f32 {
        SQRT_2
    }
}
