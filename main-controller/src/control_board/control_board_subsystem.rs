use embassy_executor::Spawner;
use embassy_stm32::gpio::Pin;
use embassy_stm32::i2c::{ErrorInterruptHandler, EventInterruptHandler, RxDma as I2cRxDma, TxDma as I2cTxDma};
use embassy_stm32::i2c::{self, mode as i2c_mode, I2c, SclPin, SdaPin};
use embassy_stm32::mode;
use embassy_stm32::peripherals::{GPDMA1_CH6, GPDMA1_CH7, PB13, PB14, UCPD1};
use embassy_stm32::spi::{self, CkPin, MckPin, MisoPin, MosiPin, RxDma, TxDma, WsPin};
use embassy_stm32::{interrupt, Peri};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use static_cell::StaticCell;

use crate::control_board::modules::audio::Audio;
use crate::control_board::tasks::{audio_tasks, ucpd_task};
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
        spi_rxsd: Peri<'static, impl MisoPin<T2>>,
        spi_ws: Peri<'static, impl WsPin<T2>>,
        spi_ck: Peri<'static, impl CkPin<T2>>,
        spi_mck: Peri<'static, impl MckPin<T2>>,
        spi_txdma: Peri<'static, impl TxDma<T2>>,
        spi_rxdma: Peri<'static, impl RxDma<T2>>,

        ucpd_peri: Peri<'static, UCPD1>,
        ucpd_cc1: Peri<'static, PB13>,
        ucpd_cc2: Peri<'static, PB14>,
        ucpd_rx_dma: Peri<'static, GPDMA1_CH6>,
        ucpd_tx_dma: Peri<'static, GPDMA1_CH7>,
    ) {
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
            spi_peri, spi_txsd, spi_rxsd, spi_ws, spi_ck, spi_mck, spi_txdma, spi_rxdma,
        );

        power_tasks::create_tasks(spawner, power_control);
        audio_tasks::create_tasks(spawner, audio).await;
        ucpd_task::create_tasks(spawner, ucpd_peri, ucpd_cc1, ucpd_cc2, ucpd_rx_dma, ucpd_tx_dma);
    }
}
