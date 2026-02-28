use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;

use crate::hardware::buffered_display::BufferedDisplay;
use crate::hardware::Displays;
use crate::state::input::MeterStateSignal;
use crate::ui;

#[embassy_executor::task]
pub async fn render_display1_task(
    fb: &'static mut BufferedDisplay,
    displays: &'static Mutex<ThreadModeRawMutex, Displays>,
    meter_state_signal: &'static MeterStateSignal,
) {
    let mut peak_dbm: i8 = -120;
    loop {
        let state = meter_state_signal.wait().await;

        if state.rssi_dbm > peak_dbm {
            peak_dbm = state.rssi_dbm;
        } else if peak_dbm > -120 {
            peak_dbm = peak_dbm.saturating_sub(1);
        }

        ui::display1::render(fb, &state, peak_dbm);

        let front = fb.swap();
        let mut d = displays.lock().await;
        let _ = d.displays[0].display.draw(front).await;
    }
}
