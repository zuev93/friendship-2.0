#![no_std]
#![no_main]

mod constants;
mod hardware;
mod state;
mod tasks;

use embassy_executor::Spawner;
use embassy_time::Timer;
use panic_halt as _;
use static_cell::StaticCell;

use crate::state::input::InputState;

static INPUT_STATE: StaticCell<InputState> = StaticCell::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let input_state = INPUT_STATE.init(InputState::new());
    let hw = hardware::init();

    tasks::buttons::spawn_tasks(&spawner, hw.buttons);
    tasks::encoders::spawn_tasks(&spawner, hw.qei_encoders, hw.exti_encoders);
    tasks::leds::spawn_tasks(&spawner, hw.leds, &input_state.leds);
    tasks::s_meter::spawn_tasks(&spawner, hw.s_meter, &input_state.s_meter);
    tasks::headphones_detect::spawn_tasks(&spawner, hw.headphones_detect);
    tasks::wm8940::spawn_tasks(&spawner, hw.wm8940, &input_state.wm8940);
    tasks::displays::spawn_tasks(
        &spawner,
        hw.displays,
        &input_state.displays,
        &input_state.displays_enabled,
    );
    tasks::spi_link::spawn_tasks(&spawner, hw.spi_link, input_state);

    loop {
        Timer::after_secs(60).await;
    }
}
