#![no_std]
#![no_main]

// Re-export common crate so it's available to all modules
extern crate druzhba_common as common;

mod app;
mod consts;
mod control_board;
mod crc;
mod front_panel;
mod hardware;
mod i2c_map;
mod main_board;
mod peripherals;

// TODO check used RSSI
// TODO add usb connection
// TODO noise reduction
// TODO DSP
// TODO Vox
// TODO CW keyer
// TODO Scan/search
// TODO add selection/changes of UI elements
// TODO add settings screen
// TODO Add system settings screen
// TODO Add FPS/UPS/RAM/CPU stats

use embassy_executor::Spawner;
use embassy_time::Timer;
use panic_halt as _;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    hardware::Hardware::init_subsystem(spawner).await;

    loop {
        Timer::after_secs(60).await;
    }
}
