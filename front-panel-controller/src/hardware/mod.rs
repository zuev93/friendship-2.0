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
    gpio::{Input, Pull},
    i2c::I2c,
    peripherals::*,
    Config,
};
use embassy_sync::blocking_mutex::{mutex::Mutex, raw::ThreadModeRawMutex};

pub struct Hardware {
    pub buttons: Buttons,
    pub encoders: Encoders,
    pub potentiometers: Potentiometers,
    pub leds: Leds,
    pub s_meter: SMeter,
    pub headphones_detect: Input<'static, PE0>,
    pub wm8940: Wm8940<I2c<'static, I2C1, DMA1_CH6, DMA1_CH5>>,
    pub spi_link: SpiLink,
    pub displays: &'static Mutex<ThreadModeRawMutex, Displays>,
}

pub fn init() -> Hardware {
    let config = Config::default();
    let p = embassy_stm32::init(config);

    Hardware {
        buttons: Buttons {
            buttons: [
                Button::new(p.PC0, p.EXTI0),
                Button::new(p.PC1, p.EXTI1),
                Button::new(p.PC2, p.EXTI2),
                Button::new(p.PC3, p.EXTI3),
                Button::new(p.PC4, p.EXTI4),
                Button::new(p.PC5, p.EXTI5),
                Button::new(p.PC6, p.EXTI6),
                Button::new(p.PC7, p.EXTI7),
                Button::new(p.PC8, p.EXTI8),
                Button::new(p.PC9, p.EXTI9),
                Button::new(p.PC10, p.EXTI10),
                Button::new(p.PC11, p.EXTI11),
            ],
        },
        encoders: Encoders {
            encoders: [
                Encoder::new(p.PC12, p.PC13, p.EXTI12, p.EXTI13),
                Encoder::new(p.PC14, p.PC15, p.EXTI14, p.EXTI15),
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
        s_meter: SMeter::new(p.DAC, p.PA4),
        headphones_detect: Input::new(p.PE0, Pull::Up),
        wm8940: wm8940::new_wm8940(p.I2C1, p.PB6, p.PB7, p.DMA1_CH6, p.DMA1_CH5),
        spi_link: SpiLink::new(p.SPI1, p.PA5, p.PB5, p.PA6, p.PA15, p.PB1),
        displays: Displays::new(
            p.SPI2, p.PB10, p.PB15, p.DMA1_CH4, p.DMA1_CH3, p.PB13, p.PB11, p.PB12, p.PB14, p.PB9,
        )
        .as_mutex(),
    }
}
