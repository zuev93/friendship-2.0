use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::Pull;
use embassy_stm32::timer::qei::{Config as QeiConfig, Qei, QeiMode};

pub enum QeiEncoder {
    Tim1(Qei<'static, embassy_stm32::peripherals::TIM1>),
    Tim2(Qei<'static, embassy_stm32::peripherals::TIM2>),
    Tim3(Qei<'static, embassy_stm32::peripherals::TIM3>),
    Tim8(Qei<'static, embassy_stm32::peripherals::TIM8>),
}

impl QeiEncoder {
    pub fn count(&self) -> u16 {
        match self {
            QeiEncoder::Tim1(q) => q.count(),
            QeiEncoder::Tim2(q) => q.count(),
            QeiEncoder::Tim3(q) => q.count(),
            QeiEncoder::Tim8(q) => q.count(),
        }
    }
}

pub fn qei_config() -> QeiConfig {
    let mut cfg = QeiConfig::default();
    cfg.ch1_pull = Pull::Up;
    cfg.ch2_pull = Pull::Up;
    cfg.mode = QeiMode::Mode3;
    cfg
}

pub struct ExtiEncoder {
    pub channel_a: ExtiInput<'static>,
    pub channel_b: ExtiInput<'static>,
}

pub struct QeiEncoders {
    pub encoders: [Option<QeiEncoder>; 4],
}

pub struct ExtiEncoders {
    pub encoders: [Option<ExtiEncoder>; 5],
}
