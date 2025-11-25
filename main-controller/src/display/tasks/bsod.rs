use crate::display::types::DisplayHardwareMutex;

#[embassy_executor::task]
pub async fn bsod_task(_hardware: DisplayHardwareMutex) {
    // TODO implement BSOD
    loop {
        embassy_time::Timer::after_secs(1).await;
    }
}
