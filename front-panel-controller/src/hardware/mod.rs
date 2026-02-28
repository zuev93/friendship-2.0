pub mod buffered_display;
mod button;
pub mod display;
mod encoder;
mod led;
mod spi_link;
mod wm8940;

pub use button::*;
pub use display::*;
pub use encoder::*;
pub use led::*;
pub use spi_link::*;

use druzhba_common::drivers::wm8940::Wm8940;
use embassy_stm32::{
    bind_interrupts,
    exti::{self, ExtiInput},
    gpio::{Input, Pull},
    i2c::{self, I2c},
    interrupt::typelevel as irqs,
    mode,
    peripherals::CRC as CRC_PERI,
    timer::qei::Qei,
    Config, Peri,
};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;

bind_interrupts!(struct ExtiIrqs {
    EXTI0 => exti::InterruptHandler<irqs::EXTI0>;
    EXTI1 => exti::InterruptHandler<irqs::EXTI1>;
    EXTI2 => exti::InterruptHandler<irqs::EXTI2>;
    EXTI3 => exti::InterruptHandler<irqs::EXTI3>;
    EXTI4 => exti::InterruptHandler<irqs::EXTI4>;
    EXTI5 => exti::InterruptHandler<irqs::EXTI5>;
    EXTI8 => exti::InterruptHandler<irqs::EXTI8>;
    EXTI9 => exti::InterruptHandler<irqs::EXTI9>;
    EXTI10 => exti::InterruptHandler<irqs::EXTI10>;
    EXTI11 => exti::InterruptHandler<irqs::EXTI11>;
    EXTI12 => exti::InterruptHandler<irqs::EXTI12>;
});

pub struct Hardware {
    pub qei_encoders: QeiEncoders,
    pub exti_encoders: ExtiEncoders,
    pub buttons: Buttons,
    pub leds: Leds,
    pub headphones_detect: ExtiInput<'static>,
    pub wm8940: Wm8940<I2c<'static, mode::Async, i2c::Master>>,
    pub spi_link: SpiLink,
    pub displays: &'static Mutex<ThreadModeRawMutex, Displays>,
    pub crc_peripheral: Peri<'static, CRC_PERI>,
}

pub fn init() -> Hardware {
    let config = Config::default();
    let p = embassy_stm32::init(config);

    Hardware {
        qei_encoders: QeiEncoders {
            encoders: [
                Some(QeiEncoder::Tim1(Qei::new(p.TIM1, p.PA8, p.PA9, qei_config()))),
                Some(QeiEncoder::Tim2(Qei::new(p.TIM2, p.PA0, p.PA1, qei_config()))),
                Some(QeiEncoder::Tim3(Qei::new(p.TIM3, p.PB4, p.PA7, qei_config()))),
                Some(QeiEncoder::Tim8(Qei::new(p.TIM8, p.PC6, p.PC7, qei_config()))),
            ],
        },
        exti_encoders: ExtiEncoders {
            encoders: [
                Some(ExtiEncoder {
                    channel_a: ExtiInput::new(p.PC0, p.EXTI0, Pull::Up, ExtiIrqs),
                    channel_b: ExtiInput::new(p.PC1, p.EXTI1, Pull::Up, ExtiIrqs),
                }),
                Some(ExtiEncoder {
                    channel_a: ExtiInput::new(p.PC2, p.EXTI2, Pull::Up, ExtiIrqs),
                    channel_b: ExtiInput::new(p.PC3, p.EXTI3, Pull::Up, ExtiIrqs),
                }),
                Some(ExtiEncoder {
                    channel_a: ExtiInput::new(p.PC4, p.EXTI4, Pull::Up, ExtiIrqs),
                    channel_b: ExtiInput::new(p.PC5, p.EXTI5, Pull::Up, ExtiIrqs),
                }),
                Some(ExtiEncoder {
                    channel_a: ExtiInput::new(p.PC8, p.EXTI8, Pull::Up, ExtiIrqs),
                    channel_b: ExtiInput::new(p.PC9, p.EXTI9, Pull::Up, ExtiIrqs),
                }),
                Some(ExtiEncoder {
                    channel_a: ExtiInput::new(p.PC10, p.EXTI10, Pull::Up, ExtiIrqs),
                    channel_b: ExtiInput::new(p.PC11, p.EXTI11, Pull::Up, ExtiIrqs),
                }),
            ],
        },
        buttons: Buttons {
            buttons: [
                Button::new(Input::new(p.PC12, Pull::Up)),
                Button::new(Input::new(p.PC13, Pull::Up)),
                Button::new(Input::new(p.PC14, Pull::Up)),
                Button::new(Input::new(p.PC15, Pull::Up)),
                Button::new(Input::new(p.PD14, Pull::Up)),
                Button::new(Input::new(p.PE2, Pull::Up)),
                Button::new(Input::new(p.PE3, Pull::Up)),
                Button::new(Input::new(p.PE4, Pull::Up)),
                Button::new(Input::new(p.PE5, Pull::Up)),
                Button::new(Input::new(p.PE6, Pull::Up)),
                Button::new(Input::new(p.PE7, Pull::Up)),
                Button::new(Input::new(p.PE8, Pull::Up)),
                Button::new(Input::new(p.PE9, Pull::Up)),
                Button::new(Input::new(p.PE10, Pull::Up)),
                Button::new(Input::new(p.PE11, Pull::Up)),
            ],
        },
        leds: Leds {
            leds: [
                Led::new(p.PD0, p.PD1),
                Led::new(p.PD2, p.PD3),
                Led::new(p.PD4, p.PD5),
                Led::new(p.PD6, p.PD7),
                Led::new(p.PD8, p.PD9),
                Led::new(p.PD10, p.PD11),
                Led::new(p.PD12, p.PD13),
            ],
        },
        headphones_detect: ExtiInput::new(p.PE12, p.EXTI12, Pull::Up, ExtiIrqs),
        wm8940: wm8940::new_wm8940(p.I2C1, p.PB6, p.PB7, p.GPDMA1_CH0, p.GPDMA1_CH1),
        spi_link: SpiLink::new(p.SPI1, p.PA5, p.PB5, p.PA6, p.PA15, p.GPDMA1_CH3, p.GPDMA1_CH4, p.PB1),
        displays: Displays::new(
            p.SPI2, p.PB10, p.PB15, p.GPDMA1_CH2, p.PB13, p.PB11, p.PB12, p.PE13, p.PB14, p.PB9,
        )
        .as_mutex(),
        crc_peripheral: p.CRC,
    }
}
