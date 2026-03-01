#![no_std]
#![no_main]

extern crate druzhba_common as common;

mod app;
mod consts;
mod control_board;
pub mod crash_info;
mod crc;
mod fault_handler;
mod front_panel;
mod hardware;
mod mpu;
mod i2c_map;
mod main_board;
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
        let bsod_err = match crash_info::ResetReason::from_u8(crash.reset_reason) {
            crash_info::ResetReason::HardFault => common::error::BsodError::CrashHardFault,
            crash_info::ResetReason::Panic => common::error::BsodError::CrashPanic,
            _ => common::error::BsodError::CrashWatchdog,
        };
        common::error::BSOD.signal(bsod_err);
    }

    loop {
        Timer::after_secs(60).await;
    }
}
