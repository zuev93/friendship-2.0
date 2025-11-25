use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use static_cell::StaticCell;

use crate::display::{
    modules::display::Display,
    types::{
        Display1Dc, Display1Spi, Display2Dc, Display2Spi, DisplayHardware, DisplayHardwareMutex,
    },
};

use super::tasks::{bsod_task, display_mode_manager};

pub struct DisplaySubsystem {
    hardware: DisplayHardwareMutex,
}

impl DisplaySubsystem {
    pub fn new(
        spi1: &'static mut Display1Spi,
        spi2: &'static mut Display2Spi,
        display1_dc: Display1Dc,
        display2_dc: Display2Dc,
    ) -> Self {
        static HARDWARE: StaticCell<Mutex<ThreadModeRawMutex, DisplayHardware>> = StaticCell::new();

        let hardware_subsys = DisplayHardware {
            display1: Display::new(spi1, display1_dc),
            display2: Display::new(spi2, display2_dc),
        };

        Self {
            hardware: HARDWARE.init(Mutex::new(hardware_subsys)),
        }
    }

    pub fn create_tasks(self, spawner: Spawner) {
        spawner.spawn(bsod_task(self.hardware)).unwrap();
        spawner.spawn(display_mode_manager(self.hardware)).unwrap();
    }
}
