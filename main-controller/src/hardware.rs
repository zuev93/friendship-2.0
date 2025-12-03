use embassy_executor::Spawner;
use embassy_stm32::{
    bind_interrupts,
    i2c::{self},
    peripherals as stm_peripherals, Config as StmConfig,
};

use crate::{
    app::AppSubsystem, control_board::control_board_subsystem::ControlBoardSybstem,
    front_panel::FrontPanelSubsystem, i2c_map::I2cMap, main_board::MainBoardSubsystem,
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

pub struct Hardware {}

impl Hardware {
    pub async fn init_subsystem(spawner: Spawner) {
        let config = StmConfig::default();
        let p = embassy_stm32::init(config);
        let irqs = Irqs;
        let i2c_map = I2cMap::new();

        MainBoardSubsystem::init_subsystem(
            spawner,
            i2c_map.main,
            irqs,
            p.I2C1,
            p.PB9,
            p.PB8,
            p.DMA1_CH6,
            p.DMA1_CH7,
            p.SPI2,
            p.PB15,
            p.PB14,
            p.PB12,
            p.PB13,
            p.PC6,
            p.DMA1_CH0,
            p.DMA1_CH1,
        )
        .await;
        FrontPanelSubsystem::init_subsystem(
            spawner, p.SPI1, p.PB5, p.PB4, p.PA5, p.DMA2_CH3, p.DMA2_CH2, p.PA4, p.PC13, p.EXTI13,
            p.SPI3, p.PB2, p.PC11, p.PA15, p.PC10, p.PC7, p.DMA2_CH0, p.DMA2_CH1,
        )
        .await;
        ControlBoardSybstem::init_subsystem(
            spawner,
            i2c_map.control_board,
            p.PB0,
            p.PB1,
            p.I2C3,
            p.PC9,
            p.PA8,
            p.SPI6,
            p.PA7,
            p.PA6,
            p.PA0,
            p.PC12,
            p.PA3,
            p.BDMA2_CH2,
            p.BDMA2_CH3,
        )
        .await;
        PeripheralsSubsystem::init_subsystem(
            spawner,
            i2c_map.peripherals,
            p.I2C4,
            p.PB7,
            p.PB6,
            p.BDMA2_CH0,
            p.BDMA2_CH1,
            irqs,
        );
        AppSubsystem::init_subsystem(spawner);
    }
}
