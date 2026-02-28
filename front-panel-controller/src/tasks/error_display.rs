use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;

use druzhba_common::error::{BSOD, ERROR_MESSAGES};

use crate::hardware::Displays;
use crate::ui::error_screen;

pub fn spawn_tasks(
    spawner: &Spawner,
    displays: &'static Mutex<ThreadModeRawMutex, Displays>,
) {
    spawner.must_spawn(error_display_task(displays));
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
) {
    loop {
        match select(BSOD.wait(), ERROR_MESSAGES.receive()).await {
            Either::First(error) => {
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
            Either::Second(message) => {
                {
                    let mut d = displays.lock().await;
                    for display in &mut d.displays {
                        error_screen::render_error(&mut display.fb, message);
                    }
                }
                flush_all(displays).await;
            }
        }
    }
}
