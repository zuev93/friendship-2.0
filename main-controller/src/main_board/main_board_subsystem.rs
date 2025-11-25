use embassy_executor::Spawner;
use embassy_stm32::{
    gpio::Pin,
    i2c::{
        self, mode as i2c_mode, ErrorInterruptHandler, EventInterruptHandler, I2c,
        RxDma as I2cRxDma, SclPin, SdaPin, TxDma as I2cTxDma,
    },
    interrupt,
    mode::{self},
    spi::{self, CkPin, MckPin, MisoPin, MosiPin, RxDma, TxDma, WsPin},
    Peri,
};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use static_cell::StaticCell;

use crate::main_board::{
    main_board::MainBoard,
    tasks::{
        audio::audio_panel_task, dds_control_task::dds_control_task,
        filter_selection::filter_selection_task, if_gain_control_task::if_gain_control_task,
        if_reference::if_reference_task, power_control::power_control_task, rssi,
    },
};

pub struct MainBoardSubsystem {}

impl MainBoardSubsystem {
    pub fn init_subsystem<T1: i2c::Instance, T2: spi::Instance>(
        spawner: Spawner,
        irqs: impl interrupt::typelevel::Binding<T1::EventInterrupt, EventInterruptHandler<T1>>
            + interrupt::typelevel::Binding<T1::ErrorInterrupt, ErrorInterruptHandler<T1>>
            + 'static,
        i2c_periph: Peri<'static, T1>,
        sda: Peri<'static, impl SdaPin<T1>>,
        scl: Peri<'static, impl SclPin<T1>>,
        i2c_txdma: Peri<'static, impl I2cTxDma<T1>>,
        i2c_rxdma: Peri<'static, impl I2cRxDma<T1>>,
        fq_ud: Peri<'static, impl Pin>,
        reset: Peri<'static, impl Pin>,
        spi_peri: Peri<'static, T2>,
        txsd: Peri<'static, impl MosiPin<T2>>,
        rxsd: Peri<'static, impl MisoPin<T2>>,
        ws: Peri<'static, impl WsPin<T2>>,
        ck: Peri<'static, impl CkPin<T2>>,
        mck: Peri<'static, impl MckPin<T2>>,
        spi_txdma: Peri<'static, impl TxDma<T2>>,
        spi_rxdma: Peri<'static, impl RxDma<T2>>,
    ) {
        static I2C1_BUS: StaticCell<
            Mutex<ThreadModeRawMutex, I2c<'static, mode::Async, i2c_mode::Master>>,
        > = StaticCell::new();

        let mut i2c_config = i2c::Config::default();
        i2c_config.sda_pullup = true;
        i2c_config.scl_pullup = true;

        let i2c1 = I2c::new(i2c_periph, scl, sda, irqs, i2c_txdma, i2c_rxdma, i2c_config);
        let i2c_mutex = I2C1_BUS.init(Mutex::new(i2c1));

        let hw = MainBoard::new(
            i2c_mutex, fq_ud, reset, spi_peri, txsd, rxsd, ws, ck, mck, spi_txdma, spi_rxdma,
        );

        rssi::spawn_tasks(spawner, hw.rssi_reader);
        spawner.must_spawn(dds_control_task(hw.dds));
        spawner.must_spawn(filter_selection_task(hw.filter_select));
        spawner.must_spawn(if_gain_control_task(hw.if_gain_control));
        // spawner.spawn(audio_control_task(hw)).; // Old, deprecated
        audio_panel_task::create_tasks(spawner, hw.audio_panel);
        spawner.must_spawn(power_control_task(hw.power_control));
        spawner.must_spawn(if_reference_task(hw.if_reference));
    }
}
