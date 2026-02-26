mod button;
mod display;
mod encoder;
mod led;
mod potentiometer;
mod s_meter;
mod spi_link;
mod wm8940;

pub use button::*;
pub use display::*;
pub use encoder::*;
pub use led::*;
pub use potentiometer::*;
pub use s_meter::*;
pub use spi_link::*;

pub mod spi_slave;

use druzhba_common::drivers::wm8940::Wm8940;
use embassy_stm32::{
    bind_interrupts,
    exti::{self, ExtiInput},
    gpio::{Input, Pull},
    i2c::{self, I2c},
    interrupt::typelevel as irqs,
    mode,
    Config,
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
    EXTI6 => exti::InterruptHandler<irqs::EXTI6>;
    EXTI7 => exti::InterruptHandler<irqs::EXTI7>;
    EXTI8 => exti::InterruptHandler<irqs::EXTI8>;
    EXTI9 => exti::InterruptHandler<irqs::EXTI9>;
    EXTI10 => exti::InterruptHandler<irqs::EXTI10>;
    EXTI11 => exti::InterruptHandler<irqs::EXTI11>;
    EXTI12 => exti::InterruptHandler<irqs::EXTI12>;
    EXTI13 => exti::InterruptHandler<irqs::EXTI13>;
    EXTI14 => exti::InterruptHandler<irqs::EXTI14>;
    EXTI15 => exti::InterruptHandler<irqs::EXTI15>;
});

pub struct Hardware {
    pub buttons: Buttons,
    pub encoders: Encoders,
    pub potentiometers: Potentiometers,
    pub leds: Leds,
    pub s_meter: SMeter,
    pub headphones_detect: Input<'static>,
    pub wm8940: Wm8940<I2c<'static, mode::Async, i2c::Master>>,
    pub spi_link: SpiLink,
    pub displays: &'static Mutex<ThreadModeRawMutex, Displays>,
}

pub fn init() -> Hardware {
    let config = Config::default();
    let p = embassy_stm32::init(config);

    Hardware {
        buttons: Buttons {
            buttons: [
                Button::new(ExtiInput::new(p.PC0, p.EXTI0, Pull::Up, ExtiIrqs)),
                Button::new(ExtiInput::new(p.PC1, p.EXTI1, Pull::Up, ExtiIrqs)),
                Button::new(ExtiInput::new(p.PC2, p.EXTI2, Pull::Up, ExtiIrqs)),
                Button::new(ExtiInput::new(p.PC3, p.EXTI3, Pull::Up, ExtiIrqs)),
                Button::new(ExtiInput::new(p.PC4, p.EXTI4, Pull::Up, ExtiIrqs)),
                Button::new(ExtiInput::new(p.PC5, p.EXTI5, Pull::Up, ExtiIrqs)),
                Button::new(ExtiInput::new(p.PC6, p.EXTI6, Pull::Up, ExtiIrqs)),
                Button::new(ExtiInput::new(p.PC7, p.EXTI7, Pull::Up, ExtiIrqs)),
                Button::new(ExtiInput::new(p.PC8, p.EXTI8, Pull::Up, ExtiIrqs)),
                Button::new(ExtiInput::new(p.PC9, p.EXTI9, Pull::Up, ExtiIrqs)),
                Button::new(ExtiInput::new(p.PC10, p.EXTI10, Pull::Up, ExtiIrqs)),
                Button::new(ExtiInput::new(p.PC11, p.EXTI11, Pull::Up, ExtiIrqs)),
            ],
        },
        encoders: Encoders {
            encoders: [
                Encoder::new(
                    ExtiInput::new(p.PC12, p.EXTI12, Pull::Up, ExtiIrqs),
                    ExtiInput::new(p.PC13, p.EXTI13, Pull::Up, ExtiIrqs),
                ),
                Encoder::new(
                    ExtiInput::new(p.PC14, p.EXTI14, Pull::Up, ExtiIrqs),
                    ExtiInput::new(p.PC15, p.EXTI15, Pull::Up, ExtiIrqs),
                ),
            ],
        },
        potentiometers: Potentiometers::new(p.ADC1, p.PA0, p.PA1, p.PA2, p.PA3, p.PA7, p.PB0),
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
        s_meter: SMeter::new(p.DAC1, p.PA4),
        headphones_detect: Input::new(p.PE0, Pull::Up),
        wm8940: wm8940::new_wm8940(p.I2C1, p.PB6, p.PB7, p.GPDMA1_CH0, p.GPDMA1_CH1),
        spi_link: SpiLink::new(p.SPI1, p.PA5, p.PB5, p.PA6, p.PA15, p.PB1),
        displays: Displays::new(
            p.SPI2, p.PB10, p.PB15, p.GPDMA1_CH2, p.PB13, p.PB11, p.PB12, p.PB14, p.PB9,
        )
        .as_mutex(),
    }
}
