use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use druzhba_common::PlatformMutex;
use embassy_sync::mutex::Mutex;

use druzhba_common::error::ERROR_MESSAGES;

use crate::hardware::Displays;
use druzhba_front_panel_controller::state::error_log::ErrorLog;
use druzhba_front_panel_controller::state::input::FatalSignal;
use druzhba_front_panel_controller::ui::error_screen;

pub fn spawn_tasks(
    spawner: &Spawner,
    displays: &'static Mutex<PlatformMutex, Displays>,
    fatal: &'static FatalSignal,
    error_log: &'static ErrorLog,
) {
    spawner.must_spawn(error_display_task(displays, fatal, error_log));
}

async fn flush_all(displays: &Mutex<PlatformMutex, Displays>) {
    let mut d = displays.lock().await;
    for display in &mut d.displays {
        let front = display.fb.swap();
        let _ = display.driver.draw(front).await;
    }
}

#[embassy_executor::task]
async fn error_display_task(
    displays: &'static Mutex<PlatformMutex, Displays>,
    fatal: &'static FatalSignal,
    error_log: &'static ErrorLog,
) {
    loop {
        match select(fatal.wait(), ERROR_MESSAGES.receive()).await {
            Either::First(error) => {
                {
                    let mut d = displays.lock().await;
                    for display in &mut d.displays {
                        error_screen::render_fatal(&mut display.fb, &error);
                    }
                }
                loop {
                    flush_all(displays).await;
                }
            }
            Either::Second(message) => {
                error_log.push(message).await;
            }
        }
    }
}
