use embassy_executor::Spawner;
use embassy_stm32::exti::{self};
use embassy_stm32::sai;
use embassy_stm32::{
    bind_interrupts,
    i2c::{self},
    interrupt::typelevel as irq_types,
    peripherals as stm_peripherals, ucpd, usb, Config as StmConfig,
};

use embassy_stm32::rcc::{
    mux, AHBPrescaler, APBPrescaler, Hse, HseMode, Hsi48Config, LsConfig, Pll, PllDiv, PllMul,
    PllPreDiv, PllSource, Sysclk, VoltageScale,
};
use embassy_stm32::time::Hertz;

use crate::{
    app::{cordic_math, AppSubsystem},
    control_board::control_board_subsystem::ControlBoardSybstem,
    front_panel::FrontPanelSubsystem,
    i2c_map::I2cMap,
    main_board::MainBoardSubsystem,
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
    USB_DRD_FS => usb::InterruptHandler<stm_peripherals::USB>;
});

pub struct Hardware {}

impl Hardware {
    pub async fn init_subsystem(spawner: Spawner) {
        let mut config = StmConfig::default();
        config.enable_ucpd1_dead_battery = true;

        config.rcc.hse = Some(Hse {
            freq: Hertz(16_000_000),
            mode: HseMode::Oscillator,
        });
        config.rcc.pll1 = Some(Pll {
            source: PllSource::HSE,
            prediv: PllPreDiv::DIV4,
            mul: PllMul::MUL125,
            divp: Some(PllDiv::DIV2),
            divq: None,
            divr: None,
        });
        config.rcc.pll2 = Some(Pll {
            source: PllSource::HSE,
            prediv: PllPreDiv::DIV5,
            mul: PllMul::MUL192,
            divp: Some(PllDiv::DIV25),
            divq: None,
            divr: None,
        });
        config.rcc.mux.spi2sel = mux::Spi2sel::PLL2_P;
        config.rcc.mux.sai1sel = mux::Saisel::PLL2_P;
        config.rcc.sys = Sysclk::PLL1_P;
        config.rcc.voltage_scale = VoltageScale::Scale0;
        config.rcc.ahb_pre = AHBPrescaler::DIV1;
        config.rcc.apb1_pre = APBPrescaler::DIV1;
        config.rcc.apb2_pre = APBPrescaler::DIV1;
        config.rcc.apb3_pre = APBPrescaler::DIV1;

        config.rcc.hsi48 = Some(Hsi48Config {
            sync_from_usb: true,
        });
        config.rcc.ls = LsConfig::default_lse();
        config.rcc.ls.enable_backup_sram = true;
        let p = embassy_stm32::init(config);

        let _swo_pin = p.PB3;

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

        FrontPanelSubsystem::init_subsystem(
            spawner,
            p.SPI1,
            p.PB5,
            p.PB4,
            p.PA5,
            p.GPDMA1_CH4,
            p.GPDMA1_CH5,
            p.PA4,
            p.PC13,
            p.EXTI13,
            irqs,
            sai2_b,
            sai2_a,
            p.PE12,
            p.PE11,
            p.PD11,
            p.PE13,
            p.PE14,
            p.GPDMA2_CH0,
            p.GPDMA2_CH1,
            p.CRC,
        )
        .await;

        ControlBoardSybstem::init_subsystem(
            spawner,
            i2c_map.control_board,
            p.PB0,
            p.PB2,
            p.PB1,
            p.I2C3,
            p.PC9,
            p.PA8,
            p.GPDMA2_CH6,
            p.GPDMA2_CH7,
            irqs,
            p.SPI2,
            p.PB15,
            p.PB12,
            p.PA9,
            p.PC6,
            p.GPDMA2_CH2,
            p.UCPD1,
            p.PB13,
            p.PB14,
            p.GPDMA1_CH6,
            p.GPDMA1_CH7,
            p.PA15,
            p.TIM2,
            p.PA0,
            p.RTC,
            p.IWDG,
            p.PD0,
            p.PD1,
            p.PC0,
            p.USB,
            p.PA12,
            p.PA11,
            irqs,
        )
        .await;
        let cordic = cordic_math::init_global(p.CORDIC);

        PeripheralsSubsystem::init_subsystem(
            spawner,
            i2c_map.peripherals,
            p.I2C4,
            p.PB7,
            p.PB6,
            p.GPDMA2_CH4,
            p.GPDMA2_CH5,
            irqs,
            cordic,
        );

        AppSubsystem::init_subsystem(spawner, cordic, p.FMAC);
    }
}
