use embassy_executor::Spawner;
use embassy_time::Timer;

use crate::app::tasks::arbiters::mode::{ModeCommand, MODE_CMD};
use crate::control_board::modules::ptt_button::PttButton;
use crate::runtime_stats::TaskId;
use druzhba_macros::instrumented;

const DEBOUNCE_THRESHOLD: u8 = 3;

pub fn create_task(spawner: Spawner, button: PttButton) {
    spawner.must_spawn(ptt_task(button));
}

#[instrumented(TaskId::Ptt)]
#[embassy_executor::task]
async fn ptt_task(button: PttButton) {
    let mut stable = false;
    let mut count: u8 = 0;

    loop {
        Timer::after_millis(5).await;

        let raw = button.pressed();
        if raw == stable {
            count = 0;
        } else {
            count += 1;
            if count >= DEBOUNCE_THRESHOLD {
                stable = raw;
                count = 0;
                if stable {
                    MODE_CMD.signal(ModeCommand::PttRearPress);
                } else {
                    MODE_CMD.signal(ModeCommand::PttRearRelease);
                }
            }
        }
    }
}
