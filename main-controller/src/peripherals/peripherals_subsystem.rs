use embassy_executor::Spawner;
use embassy_stm32::i2c::{ErrorInterruptHandler, EventInterruptHandler, RxDma, TxDma};
use embassy_stm32::{
    i2c::{self, mode as i2c_mode, I2c, SclPin, SdaPin},
    mode::{self},
};
use embassy_stm32::{interrupt, Peri};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use static_cell::StaticCell;

use crate::i2c_map::{PeripheralI2cMap};
use crate::peripherals::modules::{bpf::Bpf, hf_amp::HfAmp, lpf::Lpf};
use crate::peripherals::tasks::bpf_task::bpf_task;
use crate::peripherals::tasks::hf_amp_task::hf_amp_task;
use crate::peripherals::tasks::lpf_tasks;

pub struct PeripheralsSubsystem {}

impl PeripheralsSubsystem {
    pub fn init_subsystem<T: i2c::Instance>(
        spawner: Spawner,
        i2c_map: PeripheralI2cMap,
        i2c_periph: Peri<'static, T>,
        sda: Peri<'static, impl SdaPin<T>>,
        scl: Peri<'static, impl SclPin<T>>,
        dma_tx: Peri<'static, impl TxDma<T>>,
        dma_rx: Peri<'static, impl RxDma<T>>,
        irqs: impl interrupt::typelevel::Binding<T::EventInterrupt, EventInterruptHandler<T>>
            + interrupt::typelevel::Binding<T::ErrorInterrupt, ErrorInterruptHandler<T>>
            + 'static,
    ) {
        static I2C3_BUS: StaticCell<
            Mutex<ThreadModeRawMutex, I2c<'static, mode::Async, i2c_mode::Master>>,
        > = StaticCell::new();

        let mut i2c_config = i2c::Config::default();
        i2c_config.sda_pullup = true;
        i2c_config.scl_pullup = true;

        let i2c3 = I2c::new(i2c_periph, scl, sda, irqs, dma_tx, dma_rx, i2c_config);
        let i2c_mutex = I2C3_BUS.init(Mutex::new(i2c3));

        lpf_tasks::create_tasks(
            spawner,
            Lpf::new(i2c_mutex, i2c_map.lpf_gpio, i2c_map.lpf_ads1115),
        );
        spawner.must_spawn(bpf_task(Bpf::new(
            i2c_mutex,
            i2c_map.bpf_tca_gpio,
            i2c_map.bpf_pca_gpio,
        )));
        spawner.must_spawn(hf_amp_task(HfAmp::new(i2c_mutex, i2c_map.hf_amp_gpio)));
    }
}
