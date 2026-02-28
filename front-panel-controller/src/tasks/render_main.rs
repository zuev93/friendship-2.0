use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;

use crate::hardware::Displays;
use crate::state::input::RadioStateSignal;
use crate::ui;

pub fn spawn_tasks(
    spawner: &Spawner,
    displays: &'static Mutex<ThreadModeRawMutex, Displays>,
    radio_state_signal: &'static RadioStateSignal,
    display_index: usize,
) {
    spawner.must_spawn(render_main_task(displays, radio_state_signal, display_index));
}

#[embassy_executor::task]
async fn render_main_task(
    displays: &'static Mutex<ThreadModeRawMutex, Displays>,
    radio_state_signal: &'static RadioStateSignal,
    display_index: usize,
) {
    loop {
        let state = radio_state_signal.wait().await;

        let mut d = displays.lock().await;
        let display = &mut d.displays[display_index];
        ui::main_screen::render(&mut display.fb, &state);
        let front = display.fb.swap();
        let _ = display.driver.draw(front).await;
    }
}
