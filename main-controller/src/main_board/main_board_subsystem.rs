use embassy_executor::Spawner;
use embassy_stm32::{
    i2c::{
        self, mode as i2c_mode, ErrorInterruptHandler, EventInterruptHandler, I2c,
        RxDma as I2cRxDma, SclPin, SdaPin, TxDma as I2cTxDma,
    },
    interrupt,
    mode::{self},
    spi::{self, CkPin, MckPin, MisoPin, MosiPin, RxDma, TxDma, WsPin},
    Peri,
};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use static_cell::StaticCell;

use crate::main_board::{
    modules::{
        audio_panel::AudioPanel, crystall_filter::CrystallFilter, detector::Detector,
        if_amplifier::IfAmplifier, mixer::Mixer,
    },
    tasks::{
        audio::audio_panel_task, crystall_filter::crystall_filter_task,
        detector_tasks::detector_tasks, if_amplifier_tasks, mixer_tasks::mixer_tasks,
    },
};

pub struct MainBoardSubsystem {}

impl MainBoardSubsystem {
    pub async fn init_subsystem<T1: i2c::Instance, T2: spi::Instance>(
        spawner: Spawner,
        irqs: impl interrupt::typelevel::Binding<T1::EventInterrupt, EventInterruptHandler<T1>>
            + interrupt::typelevel::Binding<T1::ErrorInterrupt, ErrorInterruptHandler<T1>>
            + 'static,
        i2c_periph: Peri<'static, T1>,
        sda: Peri<'static, impl SdaPin<T1>>,
        scl: Peri<'static, impl SclPin<T1>>,
        i2c_txdma: Peri<'static, impl I2cTxDma<T1>>,
        i2c_rxdma: Peri<'static, impl I2cRxDma<T1>>,
        spi_peri: Peri<'static, T2>,
        txsd: Peri<'static, impl MosiPin<T2>>,
        rxsd: Peri<'static, impl MisoPin<T2>>,
        ws: Peri<'static, impl WsPin<T2>>,
        ck: Peri<'static, impl CkPin<T2>>,
        mck: Peri<'static, impl MckPin<T2>>,
        spi_txdma: Peri<'static, impl TxDma<T2>>,
        spi_rxdma: Peri<'static, impl RxDma<T2>>,
    ) {
        let mut i2c_config = i2c::Config::default();
        i2c_config.sda_pullup = true;
        i2c_config.scl_pullup = true;

        let i2c1 = I2c::new(i2c_periph, scl, sda, irqs, i2c_txdma, i2c_rxdma, i2c_config);
        static I2C1_BUS: StaticCell<
            Mutex<ThreadModeRawMutex, I2c<'static, mode::Async, i2c_mode::Master>>,
        > = StaticCell::new();
        let i2c_mutex = I2C1_BUS.init(Mutex::new(i2c1));

        spawner.must_spawn(mixer_tasks(Mixer::new(i2c_mutex)));
        if_amplifier_tasks::spawn_tasks(spawner, IfAmplifier::new(i2c_mutex));
        audio_panel_task::create_tasks(
            spawner,
            AudioPanel::new(
                i2c_mutex, spi_peri, txsd, rxsd, ws, ck, mck, spi_txdma, spi_rxdma,
            ),
        )
        .await;
        spawner.must_spawn(crystall_filter_task(CrystallFilter::new(i2c_mutex)));
        spawner.must_spawn(detector_tasks(Detector::new(i2c_mutex)));

        // TODO load settings and apply them
    }
}
