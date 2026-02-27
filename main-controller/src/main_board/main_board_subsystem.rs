use embassy_executor::Spawner;
use embassy_stm32::{
    i2c::{
        self, mode as i2c_mode, ErrorInterruptHandler, EventInterruptHandler, I2c,
        RxDma as I2cRxDma, SclPin, SdaPin, TxDma as I2cTxDma,
    },
    interrupt,
    mode::{self},
    peripherals as stm_peripherals,
    sai::{self, Dma, FsPin, MclkPin, SckPin, SdPin},
    Peri,
};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use static_cell::StaticCell;

use crate::i2c_map::MainI2cMap;
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
    pub async fn init_subsystem<T1: i2c::Instance>(
        spawner: Spawner,
        main_i2c_map: MainI2cMap,
        irqs: impl interrupt::typelevel::Binding<T1::EventInterrupt, EventInterruptHandler<T1>>
            + interrupt::typelevel::Binding<T1::ErrorInterrupt, ErrorInterruptHandler<T1>>
            + 'static,
        i2c_periph: Peri<'static, T1>,
        sda: Peri<'static, impl SdaPin<T1>>,
        scl: Peri<'static, impl SclPin<T1>>,
        i2c_txdma: Peri<'static, impl I2cTxDma<T1>>,
        i2c_rxdma: Peri<'static, impl I2cRxDma<T1>>,
        sai1_a: sai::SubBlock<'static, stm_peripherals::SAI1, sai::A>,
        sai1_b: sai::SubBlock<'static, stm_peripherals::SAI1, sai::B>,
        sai_sck: Peri<'static, impl SckPin<stm_peripherals::SAI1, sai::A>>,
        sai_sd_a: Peri<'static, impl SdPin<stm_peripherals::SAI1, sai::A>>,
        sai_sd_b: Peri<'static, impl SdPin<stm_peripherals::SAI1, sai::B>>,
        sai_fs: Peri<'static, impl FsPin<stm_peripherals::SAI1, sai::A>>,
        sai_mclk: Peri<'static, impl MclkPin<stm_peripherals::SAI1, sai::A>>,
        sai_dma_a: Peri<'static, impl Dma<stm_peripherals::SAI1, sai::A>>,
        sai_dma_b: Peri<'static, impl Dma<stm_peripherals::SAI1, sai::B>>,
    ) {
        let mut i2c_config = i2c::Config::default();
        i2c_config.sda_pullup = true;
        i2c_config.scl_pullup = true;

        let i2c1 = I2c::new(i2c_periph, scl, sda, irqs, i2c_txdma, i2c_rxdma, i2c_config);
        static I2C1_BUS: StaticCell<
            Mutex<ThreadModeRawMutex, I2c<'static, mode::Async, i2c_mode::Master>>,
        > = StaticCell::new();
        let i2c_mutex = I2C1_BUS.init(Mutex::new(i2c1));

        spawner.must_spawn(mixer_tasks(Mixer::new(
            i2c_mutex,
            main_i2c_map.mixer_sc18is602,
        )));
        if_amplifier_tasks::spawn_tasks(
            spawner,
            IfAmplifier::new(
                i2c_mutex,
                main_i2c_map.if_amp_mcp4725,
                main_i2c_map.if_amp_ads1115_rssi,
                main_i2c_map.if_amp_pca9534,
            ),
        );
        audio_panel_task::create_tasks(
            spawner,
            AudioPanel::new(
                i2c_mutex,
                main_i2c_map.audio_pcm3060,
                main_i2c_map.audio_panel_pca9534,
                sai1_a,
                sai1_b,
                sai_sck,
                sai_sd_a,
                sai_sd_b,
                sai_fs,
                sai_mclk,
                sai_dma_a,
                sai_dma_b,
            ),
        )
        .await;
        spawner.must_spawn(crystall_filter_task(CrystallFilter::new(
            i2c_mutex,
            main_i2c_map.filter_mcp4725,
            main_i2c_map.filter_pca9534,
        )));
        spawner.must_spawn(detector_tasks(Detector::new(
            i2c_mutex,
            main_i2c_map.detector_sc18is602,
        )));

        // TODO load settings and apply them
    }
}
