use embassy_executor::Spawner;
use embassy_stm32::exti::ExtiInput;

use crate::state::output::{HeadphonesEvent, OutputEvent, OUTPUT_EVENTS};

pub fn spawn_tasks(spawner: &Spawner, headphones_detect: ExtiInput<'static>) {
    spawner.must_spawn(headphones_detect_task(headphones_detect));
}

#[embassy_executor::task]
async fn headphones_detect_task(mut headphones_detect: ExtiInput<'static>) {
    let mut prev_state = headphones_detect.is_low();

    OUTPUT_EVENTS
        .send(OutputEvent::Headphones(HeadphonesEvent {
            connected: prev_state,
        }))
        .await;

    loop {
        headphones_detect.wait_for_any_edge().await;

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
