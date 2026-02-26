use druzhba_common::drivers::ssd1315::{SSD1315Config, SSD1315};
use embassy_stm32::gpio::Pin;
use embassy_stm32::{
    gpio::{Level, Output, Speed},
    mode,
    peripherals::*,
    spi::{self, Spi},
    time::Hertz,
    Peri,
};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use static_cell::StaticCell;

pub struct Display {
    pub display: SSD1315<
        Spi<'static, mode::Async, spi::mode::Master>,
        Output<'static>,
        Output<'static>,
    >,
}

impl Display {
    pub fn new(
        display: SSD1315<
            Spi<'static, mode::Async, spi::mode::Master>,
            Output<'static>,
            Output<'static>,
        >,
    ) -> Self {
        Self { display }
    }
}

pub struct Displays {
    pub displays: [Display; 2],
    pub reset: Output<'static>,
    pub backlight: Output<'static>,
}

impl Displays {
    pub fn as_mutex(self) -> &'static Mutex<ThreadModeRawMutex, Self> {
        static DISPLAYS_MUTEX: StaticCell<Mutex<ThreadModeRawMutex, Displays>> = StaticCell::new();
        DISPLAYS_MUTEX.init(Mutex::new(self))
    }

    pub fn new(
        spi2: Peri<'static, SPI2>,
        sck: Peri<'static, PB10>,
        mosi: Peri<'static, PB15>,
        tx_dma: Peri<'static, GPDMA1_CH2>,
        dc_pin: Peri<'static, impl Pin>,
        cs1_pin: Peri<'static, impl Pin>,
        cs2_pin: Peri<'static, impl Pin>,
        reset_pin: Peri<'static, impl Pin>,
        backlight_pin: Peri<'static, impl Pin>,
    ) -> Self {
        let mut spi_config = spi::Config::default();
        spi_config.frequency = Hertz(10_000_000);

        let spi2 = Spi::new_txonly(spi2, sck, mosi, tx_dma, spi_config);

        let spi2_mutex: &'static Mutex<ThreadModeRawMutex, Spi<'static, mode::Async, spi::mode::Master>> = {
            static SPI2_MUTEX: StaticCell<
                Mutex<ThreadModeRawMutex, Spi<'static, mode::Async, spi::mode::Master>>,
            > = StaticCell::new();
            SPI2_MUTEX.init(Mutex::new(spi2))
        };

        let dc = Output::new(dc_pin, Level::Low, Speed::Medium);
        let dc_mutex: &'static Mutex<ThreadModeRawMutex, Output<'static>> = {
            static DC_MUTEX: StaticCell<Mutex<ThreadModeRawMutex, Output<'static>>> =
                StaticCell::new();
            DC_MUTEX.init(Mutex::new(dc))
        };

        let cs1 = Output::new(cs1_pin, Level::High, Speed::Medium);
        let cs2 = Output::new(cs2_pin, Level::High, Speed::Medium);
        let reset = Output::new(reset_pin, Level::High, Speed::Medium);
        let backlight = Output::new(backlight_pin, Level::Low, Speed::Medium);

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
