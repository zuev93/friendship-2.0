use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;

use crate::hardware::Displays;
use crate::state::input::MeterStateSignal;
use crate::ui;

pub fn spawn_tasks(
    spawner: &Spawner,
    displays: &'static Mutex<ThreadModeRawMutex, Displays>,
    meter_state_signal: &'static MeterStateSignal,
    display_index: usize,
) {
    spawner.must_spawn(render_meter_task(displays, meter_state_signal, display_index));
}

#[embassy_executor::task]
async fn render_meter_task(
    displays: &'static Mutex<ThreadModeRawMutex, Displays>,
    meter_state_signal: &'static MeterStateSignal,
    display_index: usize,
) {
    let mut peak_dbm: i8 = -120;
    loop {
        let state = meter_state_signal.wait().await;

        if state.rssi_dbm > peak_dbm {
            peak_dbm = state.rssi_dbm;
        } else if peak_dbm > -120 {
            peak_dbm = peak_dbm.saturating_sub(1);
        }

        let mut d = displays.lock().await;
        let display = &mut d.displays[display_index];
        ui::meter_screen::render(&mut display.fb, &state, peak_dbm);
        let front = display.fb.swap();
        let _ = display.driver.draw(front).await;
    }
}
