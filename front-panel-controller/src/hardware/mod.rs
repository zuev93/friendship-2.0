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
use druzhba_common::PlatformMutex;
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
use embassy_sync::mutex::Mutex;

bind_interrupts!(struct ExtiIrqs {
    EXTI0 => exti::InterruptHandler<irqs::EXTI0>;
    EXTI1 => exti::InterruptHandler<irqs::EXTI1>;
    EXTI2 => exti::InterruptHandler<irqs::EXTI2>;
    EXTI3 => exti::InterruptHandler<irqs::EXTI3>;
    EXTI4 => exti::InterruptHandler<irqs::EXTI4>;
    EXTI5 => exti::InterruptHandler<irqs::EXTI5>;
    EXTI6 => exti::InterruptHandler<irqs::EXTI6>;
    EXTI7 => exti::InterruptHandler<irqs::EXTI7>;
    EXTI8 => exti::InterruptHandler<irqs::EXTI8>;
    EXTI9 => exti::InterruptHandler<irqs::EXTI9>;
    EXTI10 => exti::InterruptHandler<irqs::EXTI10>;
    EXTI11 => exti::InterruptHandler<irqs::EXTI11>;
    EXTI12 => exti::InterruptHandler<irqs::EXTI12>;
    EXTI14 => exti::InterruptHandler<irqs::EXTI14>;
    EXTI15 => exti::InterruptHandler<irqs::EXTI15>;
});

pub struct Hardware {
    pub qei_encoders: QeiEncoders,
    pub exti_encoders: ExtiEncoders,
    pub buttons: Buttons,
    pub leds: Leds,
    pub headphones_detect: ExtiInput<'static>,
    pub wm8940: Wm8940<I2c<'static, mode::Async, i2c::Master>>,
    pub spi_link: SpiLink,
    pub displays: &'static Mutex<PlatformMutex, Displays>,
    pub crc_peripheral: Peri<'static, CRC_PERI>,
}

pub fn init() -> Hardware {
    let config = Config::default();
    let p = embassy_stm32::init(config);

    Hardware {
        qei_encoders: QeiEncoders {
            encoders: [
                Some(QeiEncoder::Tim15(Qei::new(
                    p.TIM15,
                    p.PE5,
                    p.PE6,
                    qei_config(),
                ))), // 3.1
                Some(QeiEncoder::Tim1(Qei::new(
                    p.TIM1,
                    p.PA8,
                    p.PA9,
                    qei_config(),
                ))), // 2
                Some(QeiEncoder::Tim4(Qei::new(
                    p.TIM4,
                    p.PD13,
                    p.PD12,
                    qei_config(),
                ))), // 5.1
                Some(QeiEncoder::Tim3(Qei::new(
                    p.TIM3,
                    p.PC6,
                    p.PC7,
                    qei_config(),
                ))), // 5.2
            ],
        },
        exti_encoders: ExtiEncoders {
            encoders: [
                Some(ExtiEncoder {
                    channel_a: ExtiInput::new(p.PA15, p.EXTI15, Pull::Up, ExtiIrqs),
                    channel_b: ExtiInput::new(p.PC10, p.EXTI10, Pull::Up, ExtiIrqs),
                }), // 1.1
                Some(ExtiEncoder {
                    channel_a: ExtiInput::new(p.PD6, p.EXTI6, Pull::Up, ExtiIrqs),
                    channel_b: ExtiInput::new(p.PD7, p.EXTI7, Pull::Up, ExtiIrqs),
                }), // 1.2
                Some(ExtiEncoder {
                    channel_a: ExtiInput::new(p.PD3, p.EXTI3, Pull::Up, ExtiIrqs),
                    channel_b: ExtiInput::new(p.PD4, p.EXTI4, Pull::Up, ExtiIrqs),
                }), // 3.2
                Some(ExtiEncoder {
                    channel_a: ExtiInput::new(p.PD1, p.EXTI1, Pull::Up, ExtiIrqs),
                    channel_b: ExtiInput::new(p.PD2, p.EXTI2, Pull::Up, ExtiIrqs),
                }), // 4.1
                Some(ExtiEncoder {
                    channel_a: ExtiInput::new(p.PC11, p.EXTI11, Pull::Up, ExtiIrqs),
                    channel_b: ExtiInput::new(p.PC12, p.EXTI12, Pull::Up, ExtiIrqs),
                }), // 4.2
            ],
        },
        buttons: Buttons {
            buttons: [
                Button::new(Input::new(p.PE4, Pull::Up)),  //1
                Button::new(Input::new(p.PE3, Pull::Up)),  //2
                Button::new(Input::new(p.PE2, Pull::Up)),  //3
                Button::new(Input::new(p.PE9, Pull::Up)),  //4
                Button::new(Input::new(p.PA10, Pull::Up)), //5
                Button::new(Input::new(p.PD14, Pull::Up)), //6
                Button::new(Input::new(p.PD9, Pull::Up)),  // 7
                Button::new(Input::new(p.PA12, Pull::Up)), // encoder 1
                Button::new(Input::new(p.PC9, Pull::Up)),  // encoder 2
                Button::new(Input::new(p.PD5, Pull::Up)),  // encoder 3
                Button::new(Input::new(p.PD0, Pull::Up)),  // encoder 4
                Button::new(Input::new(p.PC8, Pull::Up)),  // encoder 5
                Button::new(Input::new(p.PB4, Pull::Up)),  // ptt
                Button::new(Input::new(p.PB8, Pull::Up)),  // sql
                Button::new(Input::new(p.PB9, Pull::Up)),  // up/down
            ],
        },
        leds: Leds {
            leds: [
                Led::new(p.PA1, p.PA2),   // 1
                Led::new(p.PC2, p.PC3),   // 2
                Led::new(p.PC13, p.PC0),  // 3
                Led::new(p.PB12, p.PE10), // 4
                Led::new(p.PA11, p.PB13), // 5
                Led::new(p.PD15, p.PD11), // 6
                Led::new(p.PD10, p.PD8),  // 7
            ],
        },
        headphones_detect: ExtiInput::new(p.PE0, p.EXTI0, Pull::Up, ExtiIrqs),
        wm8940: wm8940::new_wm8940(p.I2C1, p.PB6, p.PB7, p.GPDMA1_CH0, p.GPDMA1_CH1),
        spi_link: SpiLink::new(
            p.SPI1,
            p.PA5,
            p.PB5,
            p.PA6,
            p.PA4,
            p.GPDMA1_CH3,
            p.GPDMA1_CH4,
            p.PB1,
        ),
        displays: Displays::new(
            p.SPI2,
            p.PB10,
            p.PC1,
            p.GPDMA1_CH2,
            p.PE8,
            p.PE15,
            p.PE14,
            p.PE13,
            p.PE12,
            p.TIM2,
            p.PA0,
        )
        .as_mutex(),
        crc_peripheral: p.CRC,
    }
}
