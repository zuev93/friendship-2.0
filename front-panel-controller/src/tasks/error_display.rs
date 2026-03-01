use embassy_executor::Spawner;
use embassy_futures::select::{select3, Either3};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;

use druzhba_common::error::{BSOD, ERROR_MESSAGES};

use crate::hardware::Displays;
use crate::state::input::CrashInfoSignal;
use crate::ui::error_screen;

pub fn spawn_tasks(
    spawner: &Spawner,
    displays: &'static Mutex<ThreadModeRawMutex, Displays>,
    crash_info: &'static CrashInfoSignal,
) {
    spawner.must_spawn(error_display_task(displays, crash_info));
}

async fn flush_all(displays: &Mutex<ThreadModeRawMutex, Displays>) {
    let mut d = displays.lock().await;
    for display in &mut d.displays {
        let front = display.fb.swap();
        let _ = display.driver.draw(front).await;
    }
}

#[embassy_executor::task]
async fn error_display_task(
    displays: &'static Mutex<ThreadModeRawMutex, Displays>,
    crash_info: &'static CrashInfoSignal,
) {
    loop {
        match select3(BSOD.wait(), ERROR_MESSAGES.receive(), crash_info.wait()).await {
            Either3::First(error) => {
                {
                    let mut d = displays.lock().await;
                    for display in &mut d.displays {
                        error_screen::render_bsod(&mut display.fb, error);
                    }
                }
                loop {
                    flush_all(displays).await;
                }
            }
            Either3::Second(message) => {
                {
                    let mut d = displays.lock().await;
                    for display in &mut d.displays {
                        error_screen::render_error(&mut display.fb, message);
                    }
                }
                flush_all(displays).await;
            }
            Either3::Third(cmd) => {
                {
                    let mut d = displays.lock().await;
                    for display in &mut d.displays {
                        error_screen::render_crash_bsod(&mut display.fb, &cmd);
                    }
                }
                loop {
                    flush_all(displays).await;
                }
            }
        }
    }
}
