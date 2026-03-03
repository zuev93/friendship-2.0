use embassy_executor::Spawner;
use embassy_stm32::gpio::Pin;
use embassy_stm32::i2c::{ErrorInterruptHandler, EventInterruptHandler, RxDma as I2cRxDma, TxDma as I2cTxDma};
use embassy_stm32::i2c::{self, mode as i2c_mode, I2c, SclPin, SdaPin};
use embassy_stm32::mode;
use embassy_stm32::peripherals::{self as stm_peripherals, GPDMA1_CH6, GPDMA1_CH7, IWDG, PA0, PB13, PB14, RTC as RTC_PERI, TIM2, UCPD1};
use embassy_stm32::rtc::{Rtc, RtcConfig, RtcTimeProvider};
use embassy_stm32::spi::{self, CkPin, MosiPin, TxDma, WsPin};
use embassy_stm32::usb;
use embassy_stm32::{interrupt, Peri};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use static_cell::StaticCell;

use crate::control_board::modules::audio::Audio;
use crate::control_board::modules::cw_paddle::CwPaddle;
use crate::control_board::modules::fan::Fan;
use crate::control_board::modules::ptt_button::PttButton;
use crate::control_board::modules::usb::Usb;
use crate::control_board::modules::watchdog::Watchdog;
use crate::control_board::tasks::{audio_tasks, cw_keyer_task, fan_task, ptt_task, status_led, ucpd_task, usb_tasks, watchdog_task};
use crate::control_board::{modules::power_control::PowerControl, tasks::power_tasks};
use crate::i2c_map::ControlBoardI2cMap;

pub struct ControlBoardSybstem {}

impl ControlBoardSybstem {
    pub async fn init_subsystem<T1: i2c::Instance, T2: spi::Instance>(
        spawner: Spawner,
        i2c_map: ControlBoardI2cMap,
        pin_50v_enabled: Peri<'static, impl Pin>,
        pin_50v_mode: Peri<'static, impl Pin>,
        pin_3v3_enabled: Peri<'static, impl Pin>,
        i2c_periph: Peri<'static, T1>,
        sda: Peri<'static, impl SdaPin<T1>>,
        scl: Peri<'static, impl SclPin<T1>>,
        dma_tx: Peri<'static, impl I2cTxDma<T1>>,
        dma_rx: Peri<'static, impl I2cRxDma<T1>>,
        irqs: impl interrupt::typelevel::Binding<T1::EventInterrupt, EventInterruptHandler<T1>>
            + interrupt::typelevel::Binding<T1::ErrorInterrupt, ErrorInterruptHandler<T1>>
            + 'static,

        spi_peri: Peri<'static, T2>,
        spi_txsd: Peri<'static, impl MosiPin<T2>>,
        spi_ws: Peri<'static, impl WsPin<T2>>,
        spi_ck: Peri<'static, impl CkPin<T2>>,
        spi_txdma: Peri<'static, impl TxDma<T2>>,

        ucpd_peri: Peri<'static, UCPD1>,
        ucpd_cc1: Peri<'static, PB13>,
        ucpd_cc2: Peri<'static, PB14>,
        ucpd_rx_dma: Peri<'static, GPDMA1_CH6>,
        ucpd_tx_dma: Peri<'static, GPDMA1_CH7>,

        status_led_pin: Peri<'static, impl Pin>,

        fan_tim: Peri<'static, TIM2>,
        fan_pin: Peri<'static, PA0>,

        rtc_peri: Peri<'static, RTC_PERI>,

        iwdg_peri: Peri<'static, IWDG>,
        dit_pin: Peri<'static, impl Pin>,
        dah_pin: Peri<'static, impl Pin>,
        ptt_pin: Peri<'static, impl Pin>,

        usb_peri: Peri<'static, stm_peripherals::USB>,
        usb_dp: Peri<'static, stm_peripherals::PA12>,
        usb_dm: Peri<'static, stm_peripherals::PA11>,

        usb_irqs: impl interrupt::typelevel::Binding<
                <stm_peripherals::USB as usb::Instance>::Interrupt,
                usb::InterruptHandler<stm_peripherals::USB>,
            > + 'static,
    ) {
        static RTC_INSTANCE: StaticCell<Mutex<ThreadModeRawMutex, Rtc>> = StaticCell::new();
        static RTC_PROVIDER: StaticCell<RtcTimeProvider> = StaticCell::new();

        let (rtc, rtc_provider) = Rtc::new(rtc_peri, RtcConfig::default());
        let _rtc_mutex = RTC_INSTANCE.init(Mutex::new(rtc));
        let _provider = RTC_PROVIDER.init(rtc_provider);

        static I2C_BUS: StaticCell<
            Mutex<ThreadModeRawMutex, I2c<'static, mode::Async, i2c_mode::Master>>,
        > = StaticCell::new();

        let mut i2c_config = i2c::Config::default();
        i2c_config.sda_pullup = true;
        i2c_config.scl_pullup = true;

        let i2c = I2c::new(i2c_periph, scl, sda, irqs, dma_tx, dma_rx, i2c_config);
        let i2c_mutex = I2C_BUS.init(Mutex::new(i2c));

        let power_control = PowerControl::new(
            pin_50v_enabled,
            pin_50v_mode,
            pin_3v3_enabled,
            i2c_mutex,
            i2c_map.ina228_vbus,
            i2c_map.ina228_pa,
            i2c_map.ina228_3v3,
        );
        let audio = Audio::new(
            spi_peri, spi_txsd, spi_ws, spi_ck, spi_txdma,
        );

        let fan = Fan::new(fan_tim, fan_pin);

        status_led::create_task(spawner, status_led_pin);
        power_tasks::create_tasks(spawner, power_control);
        audio_tasks::create_tasks(spawner, audio).await;
        ucpd_task::create_tasks(spawner, ucpd_peri, ucpd_cc1, ucpd_cc2, ucpd_rx_dma, ucpd_tx_dma);
        fan_task::create_task(spawner, fan);

        let watchdog = Watchdog::new(iwdg_peri, 4_000_000);
        watchdog_task::create_task(spawner, watchdog);

        let paddle = CwPaddle::new(dit_pin, dah_pin);
        cw_keyer_task::create_task(spawner, paddle);

        let ptt = PttButton::new(ptt_pin);
        ptt_task::create_task(spawner, ptt);

        let usb = Usb::new(usb_peri, usb_dp, usb_dm, usb_irqs);
        usb_tasks::create_tasks(spawner, usb);
    }
}
