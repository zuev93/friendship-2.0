use embassy_executor::Spawner;
use embassy_stm32::gpio::Pin;
use embassy_stm32::i2c::{SclPin, SdaPin};
use embassy_stm32::peripherals as stm_peripherals;
use embassy_stm32::sai::{self, Dma, FsPin, SckPin, SdPin, SubBlockInstance};
use embassy_stm32::{i2c, Peri};

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

        sai_sub_block: sai::SubBlock<'static, stm_peripherals::SAI1, S>,
        sai_sck: Peri<'static, impl SckPin<stm_peripherals::SAI1, S>>,
        sai_sd: Peri<'static, impl SdPin<stm_peripherals::SAI1, S>>,
        sai_fs: Peri<'static, impl FsPin<stm_peripherals::SAI1, S>>,
        sai_dma: Peri<'static, impl Dma<stm_peripherals::SAI1, S>>,
    ) {
        let power_control = PowerControl::new(
            pin_13v8_enabled,
            pin_3v3_enabled,
            i2c_periph,
            sda,
            scl,
            i2c_map.ina3221,
        );
        let audio = Audio::new(sai_sub_block, sai_sck, sai_sd, sai_fs, sai_dma);

        power_tasks::create_tasks(spawner, power_control);
        audio_tasks::create_tasks(spawner, audio).await;
    }
}
