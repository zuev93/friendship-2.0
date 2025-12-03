use embassy_executor::Spawner;
use embassy_stm32::gpio::Pin;
use embassy_stm32::i2c::{SclPin, SdaPin};
use embassy_stm32::spi::{CkPin, MckPin, MisoPin, MosiPin, RxDma, TxDma, WsPin};
use embassy_stm32::{i2c, spi, Peri};

use crate::control_board::modules::audio::Audio;
use crate::control_board::tasks::audio_tasks;
use crate::control_board::{modules::power_control::PowerControl, tasks::power_tasks};
use crate::i2c_map::ControlBoardI2cMap;

pub struct ControlBoardSybstem {}

impl ControlBoardSybstem {
    pub async fn init_subsystem<T1: i2c::Instance, T2: spi::Instance>(
        spawner: Spawner,
        i2c_map: ControlBoardI2cMap,
        pin_13v8_enabled: Peri<'static, impl Pin>,
        pin_3v3_enabled: Peri<'static, impl Pin>,
        i2c_periph: Peri<'static, T1>,
        sda: Peri<'static, impl SdaPin<T1>>,
        scl: Peri<'static, impl SclPin<T1>>,

        spi_peri: Peri<'static, T2>,
        txsd: Peri<'static, impl MosiPin<T2>>,
        rxsd: Peri<'static, impl MisoPin<T2>>,
        ws: Peri<'static, impl WsPin<T2>>,
        ck: Peri<'static, impl CkPin<T2>>,
        mck: Peri<'static, impl MckPin<T2>>,
        txdma: Peri<'static, impl TxDma<T2>>,
        rxdma: Peri<'static, impl RxDma<T2>>,
    ) {
        let power_control = PowerControl::new(
            pin_13v8_enabled,
            pin_3v3_enabled,
            i2c_periph,
            sda,
            scl,
            i2c_map.ina3221,
        );
        let audio = Audio::new(spi_peri, txsd, rxsd, ws, ck, mck, txdma, rxdma);

        power_tasks::create_tasks(spawner, power_control);
        audio_tasks::create_tasks(spawner, audio).await;
    }
}
