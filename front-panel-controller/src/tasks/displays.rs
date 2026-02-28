use druzhba_common::error;
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;

use crate::{hardware::Displays, state::input::DisplaySignal};

const DISPLAY_RESET_DELAY_MS: u64 = 10;

pub fn spawn_tasks(
    spawner: &Spawner,
    displays: &'static Mutex<ThreadModeRawMutex, Displays>,
    display_signals: &'static [DisplaySignal; 2],
    displays_enabled: &'static Signal<ThreadModeRawMutex, bool>,
) {
    spawner.must_spawn(init_task(displays, displays_enabled));
    spawner.must_spawn(display_update_task(
        displays,
        display_signals,
        displays_enabled,
    ));
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
                if let Err(_) = display.display.init().await {
                    let msg = if i == 0 {
                        "Display 1 init failed"
                    } else {
                        "Display 2 init failed"
                    };
                    error::error(msg).await;
                    continue;
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

#[embassy_executor::task]
async fn display_update_task(
    displays: &'static Mutex<ThreadModeRawMutex, Displays>,
    display_signals: &'static [DisplaySignal; 2],
    displays_enabled: &'static Signal<ThreadModeRawMutex, bool>,
) {
    loop {
        let buffer0 = display_signals[0].wait();
        let buffer1 = display_signals[1].wait();

        let result = select(buffer0, buffer1).await;

        let (buffer, display_id) = match result {
            Either::First(buf) => (buf, 0),
            Either::Second(buf) => (buf, 1),
        };

        if displays_enabled.signaled() && buffer.dirty {
            let mut d = displays.lock().await;
            if let Err(_) = d.displays[display_id].display.draw(&buffer.buffer).await {
                error::error("Display draw failed").await;
            }
        }
    }
}
