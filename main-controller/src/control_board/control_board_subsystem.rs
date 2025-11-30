use embassy_executor::Spawner;
use embassy_stm32::gpio::Pin;
use embassy_stm32::i2c::{SclPin, SdaPin};
use embassy_stm32::{i2c, Peri};

use crate::control_board::{modules::power_control::PowerControl, tasks::power_tasks};

pub struct ControlBoardSybstem {}

impl ControlBoardSybstem {
    pub fn init_subsystem<T1: i2c::Instance>(
        spawner: Spawner,
        pin_13v8_enabled: Peri<'static, impl Pin>,
        pin_3v3_enabled: Peri<'static, impl Pin>,
        i2c_periph: Peri<'static, T1>,
        sda: Peri<'static, impl SdaPin<T1>>,
        scl: Peri<'static, impl SclPin<T1>>,
    ) {
        let power_control =
            PowerControl::new(pin_13v8_enabled, pin_3v3_enabled, i2c_periph, sda, scl);

        power_tasks::create_tasks(spawner, power_control);
    }
}
