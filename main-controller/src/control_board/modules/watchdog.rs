use embassy_stm32::peripherals::IWDG;
use embassy_stm32::wdg::IndependentWatchdog;
use embassy_stm32::Peri;

pub struct Watchdog {
    wdg: IndependentWatchdog<'static, IWDG>,
}

impl Watchdog {
    pub fn new(iwdg: Peri<'static, IWDG>, timeout_us: u32) -> Self {
        Self {
            wdg: IndependentWatchdog::new(iwdg, timeout_us),
        }
    }

    pub fn unleash(&mut self) {
        self.wdg.unleash();
    }

    pub fn pet(&mut self) {
        self.wdg.pet();
    }
}
