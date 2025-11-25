use embassy_stm32::{
    gpio::Output,
    peripherals::{DMA1_CH3, DMA1_CH4, PA2, PA3, SPI1},
    spi::Spi,
};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;

use crate::display::modules::display::Display;

// TODO use different SPI
pub type Display1Spi = Spi<'static, SPI1, DMA1_CH3, DMA1_CH4>;
pub type Display2Spi = Spi<'static, SPI1, DMA1_CH3, DMA1_CH4>;
pub type Display1Dc = Output<'static, PA2>;
pub type Display2Dc = Output<'static, PA3>;
pub type Display1Type = Display<Display1Spi, Display1Dc>;
pub type Display2Type = Display<Display2Spi, Display2Dc>;

#[allow(dead_code)]
pub struct DisplayHardware {
    pub display1: Display1Type,
    pub display2: Display2Type,
}

pub type DisplayHardwareMutex = &'static Mutex<ThreadModeRawMutex, DisplayHardware>;
