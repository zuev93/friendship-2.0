use druzhba_common::error;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;

use crate::hardware::Displays;

const DISPLAY_RESET_DELAY_MS: u64 = 10;

pub fn spawn_tasks(
    spawner: &Spawner,
    displays: &'static Mutex<ThreadModeRawMutex, Displays>,
    displays_enabled: &'static Signal<ThreadModeRawMutex, bool>,
) {
    spawner.must_spawn(init_task(displays, displays_enabled));
}

#[embassy_executor::task]
async fn init_task(
    displays: &'static Mutex<ThreadModeRawMutex, Displays>,
    displays_enabled: &'static Signal<ThreadModeRawMutex, bool>,
) {
    loop {
        let enabled = displays_enabled.wait().await;

        if enabled {
            let mut d = displays.lock().await;

            d.reset.set_low();
            Timer::after_millis(DISPLAY_RESET_DELAY_MS).await;
            d.reset.set_high();
            Timer::after_millis(DISPLAY_RESET_DELAY_MS).await;

            for (i, display) in d.displays.iter_mut().enumerate() {
                if let Err(_) = display.driver.init().await {
                    let msg = match i {
                        0 => "Display 1 init failed",
                        1 => "Display 2 init failed",
                        _ => "Display 3 init failed",
                    };
                    error::error(msg).await;
                }
            }

            d.backlight.set_high();
            drop(d);

            loop {
                let still_enabled = displays_enabled.wait().await;
                if !still_enabled {
                    let mut d = displays.lock().await;
                    d.backlight.set_low();
                    d.reset.set_low();
                    break;
                }
            }
        }
    }
}

