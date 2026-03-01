use embassy_executor::Spawner;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::peripherals as stm_peripherals;
use embassy_stm32::peripherals::CRC as CRC_PERI;
use embassy_stm32::sai::{self, Dma, FsPin, SckPin, SdPin};
use embassy_stm32::spi::{self, MisoPin, MosiPin, RxDma, SckPin as SpiSckPin, TxDma};
use embassy_stm32::gpio::Pin;
use embassy_stm32::Peri;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use static_cell::StaticCell;

use crate::front_panel::modules::audio::Audio;
use crate::front_panel::modules::control_bus::ControlBus;
use crate::front_panel::tasks::cursor::cursor_task;
use crate::front_panel::tasks::menu_sender::menu_sender_task;
use crate::front_panel::tasks::radio_state::radio_state_task;
use crate::front_panel::tasks::spi_receiver::spi_receiver_task;
use crate::front_panel::tasks::waterfall_sender::waterfall_sender_task;
use crate::front_panel::tasks::{
    agc_mode_led_task, audio_tasks, mode_led_task, rf_gain_mode_led_task, rit_mode_led_task,
    tone_led_task, transmit_led_task, transmit_mode_led_task,
};

pub struct FrontPanelSubsystem {}

impl FrontPanelSubsystem {
    pub async fn init_subsystem<T: spi::Instance>(
        spawner: Spawner,
        spi_bus: Peri<'static, T>,
        bus_mosi: Peri<'static, impl MosiPin<T>>,
        bus_miso: Peri<'static, impl MisoPin<T>>,
        bus_sck: Peri<'static, impl SpiSckPin<T>>,
        bus_dma_tx: Peri<'static, impl TxDma<T>>,
        bus_dma_rx: Peri<'static, impl RxDma<T>>,
        bus_cs_pin: Peri<'static, impl Pin>,
        alert_input: ExtiInput<'static>,

        sai2_b: sai::SubBlock<'static, stm_peripherals::SAI2, sai::B>,
        sai2_a: sai::SubBlock<'static, stm_peripherals::SAI2, sai::A>,
        sai_sck: Peri<'static, impl SckPin<stm_peripherals::SAI2, sai::B>>,
        sai_sd_b: Peri<'static, impl SdPin<stm_peripherals::SAI2, sai::B>>,
        sai_sd_a: Peri<'static, impl SdPin<stm_peripherals::SAI2, sai::A>>,
        sai_fs: Peri<'static, impl FsPin<stm_peripherals::SAI2, sai::B>>,
        sai_dma_b: Peri<'static, impl Dma<stm_peripherals::SAI2, sai::B>>,
        sai_dma_a: Peri<'static, impl Dma<stm_peripherals::SAI2, sai::A>>,
        crc_peripheral: Peri<'static, CRC_PERI>,
    ) {
        static SPI_LINK: StaticCell<Mutex<ThreadModeRawMutex, ControlBus>> = StaticCell::new();

        let control_bus_instance = ControlBus::new(
            spi_bus, bus_mosi, bus_miso, bus_sck, bus_dma_tx, bus_dma_rx, bus_cs_pin,
            crc_peripheral,
        );
        let bus = SPI_LINK.init(Mutex::new(control_bus_instance));

        let audio = Audio::new(
            bus,
            sai2_b,
            sai2_a,
            sai_sck,
            sai_sd_b,
            sai_sd_a,
            sai_fs,
            sai_dma_b,
            sai_dma_a,
        );
        spawner.must_spawn(spi_receiver_task(bus, alert_input));

        spawner.must_spawn(mode_led_task(bus));
        spawner.must_spawn(transmit_led_task(bus));
        spawner.must_spawn(agc_mode_led_task(bus));
        spawner.must_spawn(rf_gain_mode_led_task(bus));
        spawner.must_spawn(rit_mode_led_task(bus));
        spawner.must_spawn(tone_led_task(bus));
        spawner.must_spawn(transmit_mode_led_task(bus));
        spawner.must_spawn(radio_state_task(bus));
        spawner.must_spawn(waterfall_sender_task(bus));
        spawner.must_spawn(menu_sender_task(bus));
        spawner.must_spawn(cursor_task());
        audio_tasks::create_tasks(spawner, audio).await;
    }
}
