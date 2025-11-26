use embassy_executor::Spawner;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::{Level, Output, Pin, Pull, Speed};
use embassy_stm32::spi::{self, MisoPin, MosiPin, RxDma, SckPin, Spi, TxDma};
use embassy_stm32::time::Hertz;
use embassy_stm32::Peri;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use static_cell::StaticCell;

use crate::front_panel::modules::audio::Audio;
use crate::front_panel::modules::spi_link::SpiLink;
use crate::front_panel::tasks::{
    agc_mode_led_task, audio_task, mode_led_task, rf_gain_mode_led_task, rit_mode_led_task,
    s_meter_task, spi_receiver_task, tone_led_task, transmit_led_task, transmit_mode_led_task,
};

pub struct FrontPanelSubsystem {}

impl FrontPanelSubsystem {
    pub fn init_subsystem<T: spi::Instance, T2: Pin>(
        spawner: Spawner,
        spi_periph: Peri<'static, T>,
        mosi: Peri<'static, impl MosiPin<T>>,
        miso: Peri<'static, impl MisoPin<T>>,
        sck: Peri<'static, impl SckPin<T>>,
        dma_tx: Peri<'static, impl TxDma<T>>,
        dma_rx: Peri<'static, impl RxDma<T>>,
        cs_pin: Peri<'static, impl Pin>,
        alert_pin: Peri<'static, T2>,
        alert_exti: Peri<'static, T2::ExtiChannel>,
    ) {
        static SPI_LINK: StaticCell<Mutex<ThreadModeRawMutex, SpiLink>> = StaticCell::new();

        let mut spi_config = spi::Config::default();
        spi_config.frequency = Hertz(10_000_000);

        let spi = Spi::new(spi_periph, sck, mosi, miso, dma_tx, dma_rx, spi_config);
        let cs = Output::new(cs_pin, Level::High, Speed::High);
        let spi_link_instance = SpiLink::new(spi, cs);
        let spi_link = SPI_LINK.init(Mutex::new(spi_link_instance));

        let alert_pin = ExtiInput::new(
            alert_pin,
            alert_exti,
            // TODO check schematics
            Pull::Up,
        );
        spawner.must_spawn(spi_receiver_task(spi_link, alert_pin));

        spawner.must_spawn(mode_led_task(spi_link));
        spawner.must_spawn(transmit_led_task(spi_link));
        spawner.must_spawn(agc_mode_led_task(spi_link));
        spawner.must_spawn(rf_gain_mode_led_task(spi_link));
        spawner.must_spawn(rit_mode_led_task(spi_link));
        spawner.must_spawn(tone_led_task(spi_link));
        spawner.must_spawn(transmit_mode_led_task(spi_link));
        spawner.must_spawn(s_meter_task(spi_link));
        spawner.must_spawn(audio_task(Audio::new(spi_link)));
    }
}
