use embassy_executor::Spawner;
use embassy_stm32::exti::{self, ExtiInput};
use embassy_stm32::gpio::Pull;
use embassy_stm32::sai;
use embassy_stm32::{
    bind_interrupts,
    i2c::{self},
    interrupt::typelevel as irq_types,
    peripherals as stm_peripherals,
    ucpd,
    Config as StmConfig,
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
    EXTI13 => exti::InterruptHandler<irq_types::EXTI13>;
    UCPD1 => ucpd::InterruptHandler<stm_peripherals::UCPD1>;
});

pub struct Hardware {}

impl Hardware {
    pub async fn init_subsystem(spawner: Spawner) {
        let mut config = StmConfig::default();
        config.enable_ucpd1_dead_battery = true;
        let p = embassy_stm32::init(config);
        let irqs = Irqs;
        let i2c_map = I2cMap::new();

        let (sai1_a, sai1_b) = sai::split_subblocks(p.SAI1);
        let (sai2_a, sai2_b) = sai::split_subblocks(p.SAI2);

        MainBoardSubsystem::init_subsystem(
            spawner,
            i2c_map.main,
            irqs,
            p.I2C1,
            p.PB9,
            p.PB8,
            p.GPDMA1_CH0,
            p.GPDMA1_CH1,
            sai1_a,
            sai1_b,
            p.PE5,
            p.PE6,
            p.PE3,
            p.PE4,
            p.PE2,
            p.GPDMA1_CH2,
            p.GPDMA1_CH3,
        )
        .await;

        let alert_input = ExtiInput::new(p.PC13, p.EXTI13, Pull::Up, irqs);
        FrontPanelSubsystem::init_subsystem(
            spawner,
            p.SPI1,
            p.PB5,
            p.PB4,
            p.PA5,
            p.GPDMA1_CH4,
            p.GPDMA1_CH5,
            p.PA4,
            alert_input,
            sai2_b,
            sai2_a,
            p.PE12,
            p.PE11,
            p.PD11,
            p.PE13,
            p.GPDMA2_CH0,
            p.GPDMA2_CH1,
            p.CRC,
        )
        .await;

        ControlBoardSybstem::init_subsystem(
            spawner,
            i2c_map.control_board,
            p.PB0,
            p.PB3,
            p.PB1,
            p.I2C3,
            p.PC9,
            p.PA8,
            p.GPDMA2_CH6,
            p.GPDMA2_CH7,
            irqs,
            p.SPI2,
            p.PB15,
            p.PC2,
            p.PB12,
            p.PA9,
            p.PC6,
            p.GPDMA2_CH2,
            p.GPDMA2_CH3,
            p.UCPD1,
            p.PB13,
            p.PB14,
            p.GPDMA1_CH6,
            p.GPDMA1_CH7,
        )
        .await;
        PeripheralsSubsystem::init_subsystem(
            spawner,
            i2c_map.peripherals,
            p.I2C4,
            p.PB7,
            p.PB6,
            p.GPDMA2_CH4,
            p.GPDMA2_CH5,
            irqs,
        );
        AppSubsystem::init_subsystem(spawner);
    }
}
