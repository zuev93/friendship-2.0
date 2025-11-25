use druzhba_common::drivers::ssd1315::{SSD1315Config, SSD1315};
use embassy_stm32::gpio::Pin;
use embassy_stm32::{
    gpio::{AnyPin, Level, Output, Speed},
    peripherals::*,
    spi::{self, Spi},
    time::Hertz,
    Peripheral,
};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use static_cell::StaticCell;

pub struct Display {
    pub display: SSD1315<
        Spi<'static, SPI2, DMA1_CH4, DMA1_CH3>,
        Output<'static, AnyPin>,
        Output<'static, AnyPin>,
    >,
}

impl Display {
    pub fn new(
        display: SSD1315<
            Spi<'static, SPI2, DMA1_CH4, DMA1_CH3>,
            Output<'static, AnyPin>,
            Output<'static, AnyPin>,
        >,
    ) -> Self {
        Self { display }
    }
}

pub struct Displays {
    pub displays: [Display; 2],
    pub reset: Output<'static, AnyPin>,
    pub backlight: Output<'static, AnyPin>,
}

impl Displays {
    pub fn as_mutex(self) -> &'static Mutex<ThreadModeRawMutex, Self> {
        static DISPLAYS_MUTEX: StaticCell<Mutex<ThreadModeRawMutex, Displays>> = StaticCell::new();
        DISPLAYS_MUTEX.init(Mutex::new(self))
    }

    pub fn new(
        spi2: SPI2,
        sck: PB10,
        mosi: PB15,
        tx_dma: DMA1_CH4,
        rx_dma: DMA1_CH3,
        dc_pin: impl Peripheral<P = impl Pin> + 'static,
        cs1_pin: impl Peripheral<P = impl Pin> + 'static,
        cs2_pin: impl Peripheral<P = impl Pin> + 'static,
        reset_pin: impl Peripheral<P = impl Pin> + 'static,
        backlight_pin: impl Peripheral<P = impl Pin> + 'static,
    ) -> Self {
        let mut spi_config = spi::Config::default();
        spi_config.frequency = Hertz(10_000_000);

        let spi2 = Spi::new_txonly(spi2, sck, mosi, tx_dma, rx_dma, spi_config);

        let spi2_mutex: &'static Mutex<ThreadModeRawMutex, Spi<'static, SPI2, DMA1_CH4, DMA1_CH3>> = {
            static SPI2_MUTEX: StaticCell<
                Mutex<ThreadModeRawMutex, Spi<'static, SPI2, DMA1_CH4, DMA1_CH3>>,
            > = StaticCell::new();
            SPI2_MUTEX.init(Mutex::new(spi2))
        };

        let dc = Output::new(dc_pin, Level::Low, Speed::Medium).degrade();
        let dc_mutex: &'static Mutex<ThreadModeRawMutex, Output<'static, AnyPin>> = {
            static DC_MUTEX: StaticCell<Mutex<ThreadModeRawMutex, Output<'static, AnyPin>>> =
                StaticCell::new();
            DC_MUTEX.init(Mutex::new(dc))
        };

        let cs1 = Output::new(cs1_pin, Level::High, Speed::Medium).degrade();
        let cs2 = Output::new(cs2_pin, Level::High, Speed::Medium).degrade();
        let reset = Output::new(reset_pin, Level::High, Speed::Medium).degrade();
        let backlight = Output::new(backlight_pin, Level::Low, Speed::Medium).degrade();

        let display1 = Display::new(SSD1315::new(
            spi2_mutex,
            dc_mutex,
            cs1,
            SSD1315Config::default(),
        ));
        let display2 = Display::new(SSD1315::new(
            spi2_mutex,
            dc_mutex,
            cs2,
            SSD1315Config::default(),
        ));

        Self {
            displays: [display1, display2],
            reset,
            backlight,
        }
    }
}
