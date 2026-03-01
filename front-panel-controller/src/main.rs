#![no_std]
#![no_main]

mod constants;
mod crc;
mod hardware;
mod state;
mod tasks;
mod ui;

use embassy_executor::Spawner;
use embassy_time::Timer;
use panic_halt as _;
use static_cell::StaticCell;

use crate::crc::HardwareCrc16Modbus;
use crate::state::input::InputState;

static INPUT_STATE: StaticCell<InputState> = StaticCell::new();
static HW_CRC: StaticCell<HardwareCrc16Modbus> = StaticCell::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let input_state = INPUT_STATE.init(InputState::new());
    let hw = hardware::init();
    let hw_crc = HW_CRC.init(HardwareCrc16Modbus::new(hw.crc_peripheral));

    tasks::buttons::spawn_tasks(&spawner, hw.buttons);
    tasks::encoders::spawn_tasks(&spawner, hw.qei_encoders, hw.exti_encoders);
    tasks::leds::spawn_tasks(&spawner, hw.leds, &input_state.leds);
    tasks::headphones_detect::spawn_tasks(&spawner, hw.headphones_detect);
    tasks::wm8940::spawn_tasks(&spawner, hw.wm8940, &input_state.wm8940);
    tasks::displays::spawn_tasks(&spawner, hw.displays);
    tasks::spi_link::spawn_tasks(&spawner, hw.spi_link, input_state, hw_crc);
    tasks::render_meter::spawn_tasks(&spawner, hw.displays, &input_state.radio_state, 0);
    tasks::render_spectrum::spawn_tasks(&spawner, hw.displays, &input_state.waterfall_line, 1);
    tasks::menu::spawn_tasks(&spawner, &input_state.menu_screen);
    tasks::render_main::spawn_tasks(&spawner, hw.displays, &input_state.radio_state, &input_state.menu_screen, 2);
    tasks::error_display::spawn_tasks(&spawner, hw.displays);
    tasks::fps_task::spawn_tasks(&spawner);

    loop {
        Timer::after_secs(60).await;
    }
}
