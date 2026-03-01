use embassy_stm32::pac;
use embassy_stm32::peripherals::FMAC;
use embassy_stm32::{rcc, Peri};

pub const FIR_TAPS: usize = 127;

const X2_BASE: u8 = 0;
const X2_SIZE: u8 = 127;
const X1_BASE: u8 = 127;
const X1_SIZE: u8 = 128;
const Y_BASE: u8 = 255;
const Y_SIZE: u8 = 1;

const FUNC_LOAD_X1: u8 = 0x00;
const FUNC_LOAD_X2: u8 = 0x01;
const FUNC_FIR: u8 = 0x08;

fn f32_to_q15(x: f32) -> i16 {
    (x * 32767.0).clamp(-32768.0, 32767.0) as i16
}

pub struct FmacFir {
    _peri: Peri<'static, FMAC>,
}

impl FmacFir {
    pub fn new(peri: Peri<'static, FMAC>) -> Self {
        rcc::enable_and_reset::<FMAC>();

        let fmac = pac::FMAC;

        fmac.cr().modify(|w| w.set_reset(true));
        while fmac.cr().read().reset() {}

        fmac.cr().modify(|w| w.set_clipen(true));

        fmac.x1bufcfg().write(|w| {
            w.set_x1_base(X1_BASE);
            w.set_x1_buf_size(X1_SIZE);
            w.set_full_wm(0);
        });

        fmac.x2bufcfg().write(|w| {
            w.set_x2_base(X2_BASE);
            w.set_x2_buf_size(X2_SIZE);
        });

        fmac.ybufcfg().write(|w| {
            w.set_y_base(Y_BASE);
            w.set_y_buf_size(Y_SIZE);
            w.set_empty_wm(0);
        });

        Self { _peri: peri }
    }

    pub fn load_coefficients(&mut self, coeffs: &[f32; FIR_TAPS]) {
        let fmac = pac::FMAC;

        fmac.param().write(|w| {
            w.set_p(X2_SIZE);
            w.set_func(FUNC_LOAD_X2);
            w.set_start(true);
        });

        for &c in coeffs.iter() {
            let q15 = f32_to_q15(c) as u16;
            fmac.wdata().write_value(pac::fmac::regs::Wdata(q15 as u32));
        }

        fmac.param().write(|w| {
            w.set_p(X1_SIZE - 1);
            w.set_func(FUNC_LOAD_X1);
            w.set_start(true);
        });

        for _ in 0..(X1_SIZE - 1) {
            fmac.wdata().write_value(pac::fmac::regs::Wdata(0));
        }
    }

    pub fn start_fir(&mut self) {
        let fmac = pac::FMAC;
        fmac.param().write(|w| {
            w.set_p(X2_SIZE);
            w.set_q(0);
            w.set_r(1);
            w.set_func(FUNC_FIR);
            w.set_start(true);
        });
    }

    pub fn stop(&mut self) {
        let fmac = pac::FMAC;
        fmac.cr().modify(|w| w.set_reset(true));
        while fmac.cr().read().reset() {}

        fmac.cr().modify(|w| w.set_clipen(true));

        fmac.x1bufcfg().write(|w| {
            w.set_x1_base(X1_BASE);
            w.set_x1_buf_size(X1_SIZE);
            w.set_full_wm(0);
        });

        fmac.x2bufcfg().write(|w| {
            w.set_x2_base(X2_BASE);
            w.set_x2_buf_size(X2_SIZE);
        });

        fmac.ybufcfg().write(|w| {
            w.set_y_base(Y_BASE);
            w.set_y_buf_size(Y_SIZE);
            w.set_empty_wm(0);
        });
    }

    pub fn process_sample(&mut self, sample: i16) -> i16 {
        let fmac = pac::FMAC;
        fmac.wdata()
            .write_value(pac::fmac::regs::Wdata(sample as u16 as u32));
        while fmac.sr().read().yempty() {}
        fmac.rdata().read().res() as i16
    }
}
