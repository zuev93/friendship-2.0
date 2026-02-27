use embassy_executor::Spawner;
use embassy_stm32::gpio::Pin;
use embassy_stm32::i2c::{ErrorInterruptHandler, EventInterruptHandler, RxDma, TxDma};
use embassy_stm32::i2c::{self, mode as i2c_mode, I2c, SclPin, SdaPin};
use embassy_stm32::mode;
use embassy_stm32::peripherals as stm_peripherals;
use embassy_stm32::sai::{self, Dma, FsPin, SckPin, SdPin, SubBlockInstance};
use embassy_stm32::{interrupt, Peri};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use static_cell::StaticCell;

use crate::control_board::modules::audio::Audio;
use crate::control_board::tasks::audio_tasks;
use crate::control_board::{modules::power_control::PowerControl, tasks::power_tasks};
use crate::i2c_map::ControlBoardI2cMap;

pub struct ControlBoardSybstem {}

impl ControlBoardSybstem {
    pub async fn init_subsystem<T1: i2c::Instance, S: SubBlockInstance>(
        spawner: Spawner,
        i2c_map: ControlBoardI2cMap,
        pin_13v8_enabled: Peri<'static, impl Pin>,
        pin_3v3_enabled: Peri<'static, impl Pin>,
        i2c_periph: Peri<'static, T1>,
        sda: Peri<'static, impl SdaPin<T1>>,
        scl: Peri<'static, impl SclPin<T1>>,
        dma_tx: Peri<'static, impl TxDma<T1>>,
        dma_rx: Peri<'static, impl RxDma<T1>>,
        irqs: impl interrupt::typelevel::Binding<T1::EventInterrupt, EventInterruptHandler<T1>>
            + interrupt::typelevel::Binding<T1::ErrorInterrupt, ErrorInterruptHandler<T1>>
            + 'static,

        sai_sub_block: sai::SubBlock<'static, stm_peripherals::SAI1, S>,
        sai_sck: Peri<'static, impl SckPin<stm_peripherals::SAI1, S>>,
        sai_sd: Peri<'static, impl SdPin<stm_peripherals::SAI1, S>>,
        sai_fs: Peri<'static, impl FsPin<stm_peripherals::SAI1, S>>,
        sai_dma: Peri<'static, impl Dma<stm_peripherals::SAI1, S>>,
    ) {
        static I2C_BUS: StaticCell<
            Mutex<ThreadModeRawMutex, I2c<'static, mode::Async, i2c_mode::Master>>,
        > = StaticCell::new();

        let mut i2c_config = i2c::Config::default();
        i2c_config.sda_pullup = true;
        i2c_config.scl_pullup = true;

        let i2c = I2c::new(i2c_periph, scl, sda, irqs, dma_tx, dma_rx, i2c_config);
        let i2c_mutex = I2C_BUS.init(Mutex::new(i2c));

        let power_control = PowerControl::new(
            pin_13v8_enabled,
            pin_3v3_enabled,
            i2c_mutex,
            i2c_map.ina228_vbus,
            i2c_map.ina228_pa,
            i2c_map.ina228_3v3,
        );
        let audio = Audio::new(sai_sub_block, sai_sck, sai_sd, sai_fs, sai_dma);

        power_tasks::create_tasks(spawner, power_control);
        audio_tasks::create_tasks(spawner, audio).await;
    }
}
