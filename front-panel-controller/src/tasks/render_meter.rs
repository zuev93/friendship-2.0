use embassy_executor::Spawner;
use druzhba_common::PlatformMutex;
use embassy_sync::mutex::Mutex;

use crate::hardware::Displays;
use druzhba_front_panel_controller::state::input::RadioStateSignal;
use druzhba_front_panel_controller::ui;

pub fn spawn_tasks(
    spawner: &Spawner,
    displays: &'static Mutex<PlatformMutex, Displays>,
    radio_state_signal: &'static RadioStateSignal,
    display_index: usize,
) {
    spawner.must_spawn(render_meter_task(displays, radio_state_signal, display_index));
}

#[embassy_executor::task]
async fn render_meter_task(
    displays: &'static Mutex<PlatformMutex, Displays>,
    radio_state_signal: &'static RadioStateSignal,
    display_index: usize,
) {
    let mut peak_dbm: i8 = -120;
    loop {
        let state = radio_state_signal.wait().await;

        if state.rssi_dbm > peak_dbm {
            peak_dbm = state.rssi_dbm;
        } else if peak_dbm > -120 {
            peak_dbm = peak_dbm.saturating_sub(1);
        }

        let mut d = displays.lock().await;
        let display = &mut d.displays[display_index];
        ui::meter_screen::render(&mut display.fb, &state, peak_dbm);
        let front = display.fb.swap();
        if display.driver.draw(front).await.is_ok() {
            display.count_frame();
        }
    }
}
