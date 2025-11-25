use embassy_executor::Spawner;
use embassy_stm32::gpio::Input;
use embassy_stm32::peripherals::PE0;
use embassy_time::Timer;

use crate::constants::HEADPHONES_POLL_INTERVAL;
use crate::state::output::{HeadphonesEvent, OutputEvent, OUTPUT_EVENTS};

pub fn spawn_tasks(spawner: &Spawner, headphones_detect: Input<'static, PE0>) {
    spawner.must_spawn(headphones_detect_task(headphones_detect));
}

#[embassy_executor::task]
async fn headphones_detect_task(headphones_detect: Input<'static, PE0>) {
    let mut prev_state = headphones_detect.is_low();

    loop {
        Timer::after(HEADPHONES_POLL_INTERVAL).await;

        let current_state = headphones_detect.is_low();

        if current_state != prev_state {
            OUTPUT_EVENTS
                .send(OutputEvent::Headphones(HeadphonesEvent {
                    connected: current_state,
                }))
                .await;
            prev_state = current_state;
        }
    }
}
