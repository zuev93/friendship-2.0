use embassy_executor::Spawner;
use embassy_stm32::{
    bind_interrupts,
    i2c::{self},
    peripherals as stm_peripherals, Config as StmConfig,
};

use crate::{
    app::AppSubsystem, front_panel::FrontPanelSubsystem, main_board::MainBoardSubsystem,
    peripherals::peripherals_subsystem::PeripheralsSubsystem,
};

bind_interrupts!(pub struct Irqs {
    I2C1_EV => i2c::EventInterruptHandler<stm_peripherals::I2C1>;
    I2C1_ER => i2c::ErrorInterruptHandler<stm_peripherals::I2C1>;
    I2C3_EV => i2c::EventInterruptHandler<stm_peripherals::I2C3>;
    I2C3_ER => i2c::ErrorInterruptHandler<stm_peripherals::I2C3>;
    I2C4_EV => i2c::EventInterruptHandler<stm_peripherals::I2C4>;
    I2C4_ER => i2c::ErrorInterruptHandler<stm_peripherals::I2C4>;
});

pub struct Hardware {
    pub front_panel: FrontPanelSubsystem,
    pub app: AppSubsystem,
}

impl Hardware {
    // TODO rename me
    pub fn new(spawner: Spawner) -> Self {
        let config = StmConfig::default();
        let p = embassy_stm32::init(config);
        let irqs = Irqs;

        let front_panel = FrontPanelSubsystem::new(
            p.SPI1, p.PA7, p.PA6, p.PA5, p.DMA2_CH3, p.DMA2_CH2, p.PA4, p.PB5, p.EXTI5,
        );
        MainBoardSubsystem::init_subsystem(
            spawner, irqs, p.I2C1, p.PB9, p.PB8, p.DMA1_CH6, p.DMA1_CH7, p.PA0, p.PA1, p.SPI2,
            p.PB15, p.PB14, p.PB12, p.PB13, p.PC6, p.DMA1_CH0, p.DMA1_CH1,
        );
        let app = AppSubsystem::new();
        PeripheralsSubsystem::init_subsystem(
            spawner, p.I2C3, p.PC9, p.PA8, p.DMA1_CH4, p.DMA1_CH2, irqs,
        );

        Self { front_panel, app }
    }
}
