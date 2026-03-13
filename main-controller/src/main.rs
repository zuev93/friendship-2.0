#![no_std]
#![no_main]

extern crate druzhba_common as common;

mod app;
mod control_board;
use druzhba_main_controller::consts;
use druzhba_main_controller::cordic_math;
use druzhba_main_controller::dsp;
pub mod crash_info;
mod crc;
mod fault_handler;
mod front_panel;
mod hardware;
mod i2c_map;
mod main_board;
mod mpu;
mod panic_handler;
mod peripherals;
pub mod runtime_stats;

// TODO add settings screen
// TODO Add system settings screen
// TODO AM mode (and other modes)
// TODO add firmware update via usb.
// TODO tests
// EEPROM settings
// TODO Number of buttons and encoders/config is error prone.
// TODO move logic out of audio mixer - it is a mixer, not a dsp engine

use core::sync::atomic::Ordering;
use embassy_executor::Spawner;
use embassy_time::{Duration, Instant, Timer};

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut cp = cortex_m::Peripherals::take().unwrap();
    cp.DCB.enable_trace();
    cp.DWT.enable_cycle_counter();

    mpu::init_stack_guard();

    let cal_start = Instant::now();
    let mut cal_count: u32 = 0;
    while Instant::now().duration_since(cal_start) < Duration::from_millis(100) {
        cal_count += 1;
    }
    let idle_max_per_sec = cal_count.saturating_mul(10);
    runtime_stats::IDLE_CAL.store(idle_max_per_sec, Ordering::Relaxed);

    hardware::Hardware::init_subsystem(spawner).await;

    if let Some(crash) = crash_info::check_and_take() {
        crash_info::LAST_CRASH.sender().send(crash);
        common::error::BSOD.signal(common::error::BsodError::Crash);
    }

    loop {
        Timer::after_secs(60).await;
    }
}
