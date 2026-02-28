use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Timer;

use crate::hardware::Displays;
use crate::state::input::WaterfallLineSignal;
use crate::ui;
use crate::ui::spectrum_screen::WaterfallDisplayBuffer;

pub fn spawn_tasks(
    spawner: &Spawner,
    displays: &'static Mutex<ThreadModeRawMutex, Displays>,
    waterfall_signal: &'static WaterfallLineSignal,
    display_index: usize,
) {
    spawner.must_spawn(render_spectrum_task(displays, waterfall_signal, display_index));
}

#[embassy_executor::task]
async fn render_spectrum_task(
    displays: &'static Mutex<ThreadModeRawMutex, Displays>,
    waterfall_signal: &'static WaterfallLineSignal,
    display_index: usize,
) {
    let mut buf = WaterfallDisplayBuffer::new();

    loop {
        let stale = match select(waterfall_signal.wait(), Timer::after_millis(500)).await {
            Either::First(data) => {
                buf.push(&data);
                false
            }
            Either::Second(_) => true,
        };

        let mut d = displays.lock().await;
        let display = &mut d.displays[display_index];
        ui::spectrum_screen::render(&mut display.fb, &buf, stale);
        let front = display.fb.swap();
        let _ = display.driver.draw(front).await;
    }
}
