use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use druzhba_common::PlatformMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::Timer;

use crate::hardware::Displays;
use druzhba_front_panel_controller::state::input::WaterfallLineSignal;
use druzhba_front_panel_controller::ui;
use druzhba_front_panel_controller::ui::spectrum_screen::WaterfallDisplayBuffer;

pub fn spawn_tasks(
    spawner: &Spawner,
    displays: &'static Mutex<PlatformMutex, Displays>,
    waterfall_signal: &'static WaterfallLineSignal,
    display_index: usize,
) {
    spawner.must_spawn(render_spectrum_task(displays, waterfall_signal, display_index));
}

#[embassy_executor::task]
async fn render_spectrum_task(
    displays: &'static Mutex<PlatformMutex, Displays>,
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
        if display.driver.draw(front).await.is_ok() {
            display.count_frame();
        }
    }
}
