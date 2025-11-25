use embassy_executor::Spawner;
use embassy_time::{with_timeout, TimeoutError};

use crate::constants::{BUTTON_DEBOUNCE_DURATION, BUTTON_MAX_DEBOUNCE_RETRIES};
use crate::hardware::{Button, Buttons};
use crate::state::output::{ButtonEvent, ButtonState, OutputEvent, OUTPUT_EVENTS};

pub fn spawn_tasks(spawner: &Spawner, mut buttons: Buttons) {
    for i in 0..buttons.buttons.len() {
        if let Some(button) = buttons.buttons[i].take() {
            spawner.must_spawn(button_task(i as u8, button));
        }
    }
}

#[embassy_executor::task]
async fn button_task(button_id: u8, mut button: Button) {
    loop {
        button.pin.wait_for_any_edge().await;

        let mut retries = 0;

        loop {
            match with_timeout(BUTTON_DEBOUNCE_DURATION, button.pin.wait_for_any_edge()).await {
                Ok(_) => {
                    retries += 1;
                    if retries >= BUTTON_MAX_DEBOUNCE_RETRIES {
                        break;
                    }
                }
                Err(TimeoutError) => break,
            }
        }

        if retries < BUTTON_MAX_DEBOUNCE_RETRIES {
            let state = if button.pin.is_low() {
                ButtonState::Pressed
            } else {
                ButtonState::Released
            };

            OUTPUT_EVENTS
                .send(OutputEvent::Button(ButtonEvent {
                    id: button_id,
                    state,
                }))
                .await;
        }
    }
}
