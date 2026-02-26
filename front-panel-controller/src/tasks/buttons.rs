use embassy_executor::Spawner;
use embassy_time::Timer;

use crate::constants::BUTTON_POLL_INTERVAL;
use crate::hardware::Buttons;
use crate::state::output::{ButtonEvent, ButtonState, OutputEvent, OUTPUT_EVENTS};

const DEBOUNCE_COUNT: u8 = 4;

pub fn spawn_tasks(spawner: &Spawner, buttons: Buttons) {
    spawner.must_spawn(button_poll_task(buttons));
}

#[embassy_executor::task]
async fn button_poll_task(buttons: Buttons) {
    let mut stable_state = [false; 15];
    let mut counter = [0u8; 15];

    for (i, btn) in buttons.buttons.iter().enumerate() {
        if let Some(b) = btn {
            stable_state[i] = b.is_low();
        }
    }

    loop {
        Timer::after(BUTTON_POLL_INTERVAL).await;

        for (i, btn) in buttons.buttons.iter().enumerate() {
            let b = match btn {
                Some(b) => b,
                None => continue,
            };

            let current = b.is_low();

            if current != stable_state[i] {
                counter[i] += 1;
                if counter[i] >= DEBOUNCE_COUNT {
                    stable_state[i] = current;
                    counter[i] = 0;

                    let state = if current {
                        ButtonState::Pressed
                    } else {
                        ButtonState::Released
                    };

                    OUTPUT_EVENTS
                        .send(OutputEvent::Button(ButtonEvent {
                            id: i as u8,
                            state,
                        }))
                        .await;
                }
            } else {
                counter[i] = 0;
            }
        }
    }
}
