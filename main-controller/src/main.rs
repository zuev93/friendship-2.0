#![no_std]
#![no_main]

// Re-export common crate so it's available to all modules
extern crate druzhba_common as common;

mod app;
mod consts;
mod front_panel;
mod hardware;
mod i2c_map;
mod main_board;
mod peripherals;

use embassy_executor::Spawner;
use embassy_time::Timer;
use panic_halt as _;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let hw = hardware::Hardware::new(spawner);

    hw.front_panel.create_tasks(spawner);
    hw.app.create_tasks(spawner);

    loop {
        Timer::after_secs(60).await;
    }
}
