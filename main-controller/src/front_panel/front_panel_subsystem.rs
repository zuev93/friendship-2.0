use embassy_executor::Spawner;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::{Pin, Pull};
use embassy_stm32::spi::{self, CkPin, MckPin, MisoPin, MosiPin, RxDma, SckPin, TxDma, WsPin};
use embassy_stm32::Peri;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use static_cell::StaticCell;

use crate::front_panel::modules::audio::Audio;
use crate::front_panel::modules::control_bus::ControlBus;
use crate::front_panel::tasks::s_meter::s_meter_task;
use crate::front_panel::tasks::spi_receiver::spi_receiver_task;
use crate::front_panel::tasks::{
    agc_mode_led_task, audio_tasks, mode_led_task, rf_gain_mode_led_task, rit_mode_led_task,
    tone_led_task, transmit_led_task, transmit_mode_led_task,
};

pub struct FrontPanelSubsystem {}

impl FrontPanelSubsystem {
    pub async fn init_subsystem<T: spi::Instance, T2: Pin, T3: spi::Instance>(
        spawner: Spawner,
        spi_bus: Peri<'static, T>,
        bus_mosi: Peri<'static, impl MosiPin<T>>,
        bus_miso: Peri<'static, impl MisoPin<T>>,
        bus_sck: Peri<'static, impl SckPin<T>>,
        bus_dma_tx: Peri<'static, impl TxDma<T>>,
        bus_dma_rx: Peri<'static, impl RxDma<T>>,
        bus_cs_pin: Peri<'static, impl Pin>,
        bus_alert_pin: Peri<'static, T2>,
        bus_alert_exti: Peri<'static, T2::ExtiChannel>,

        spi_audio: Peri<'static, T3>,
        audio_txsd: Peri<'static, impl MosiPin<T3>>,
        audio_rxsd: Peri<'static, impl MisoPin<T3>>,
        audio_ws: Peri<'static, impl WsPin<T3>>,
        audio_ck: Peri<'static, impl CkPin<T3>>,
        audio_mck: Peri<'static, impl MckPin<T3>>,
        audio_txdma: Peri<'static, impl TxDma<T3>>,
        audio_rxdma: Peri<'static, impl RxDma<T3>>,
    ) {
        static SPI_LINK: StaticCell<Mutex<ThreadModeRawMutex, ControlBus>> = StaticCell::new();

        let control_bus_instance = ControlBus::new(
            spi_bus, bus_mosi, bus_miso, bus_sck, bus_dma_tx, bus_dma_rx, bus_cs_pin,
        );
        let bus = SPI_LINK.init(Mutex::new(control_bus_instance));

        let alert_pin = ExtiInput::new(
            bus_alert_pin,
            bus_alert_exti,
            // TODO check schematics
            Pull::Up,
        );
        let audio = Audio::new(
            bus,
            spi_audio,
            audio_txsd,
            audio_rxsd,
            audio_ws,
            audio_ck,
            audio_mck,
            audio_txdma,
            audio_rxdma,
        );
        spawner.must_spawn(spi_receiver_task(bus, alert_pin));

        spawner.must_spawn(mode_led_task(bus));
        spawner.must_spawn(transmit_led_task(bus));
        spawner.must_spawn(agc_mode_led_task(bus));
        spawner.must_spawn(rf_gain_mode_led_task(bus));
        spawner.must_spawn(rit_mode_led_task(bus));
        spawner.must_spawn(tone_led_task(bus));
        spawner.must_spawn(transmit_mode_led_task(bus));
        spawner.must_spawn(s_meter_task(bus));
        audio_tasks::create_tasks(spawner, audio).await;
    }
}
