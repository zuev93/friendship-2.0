pub struct EmbassySinkTimer;

impl usbpd::timers::Timer for EmbassySinkTimer {
    async fn after_millis(milliseconds: u64) {
        embassy_time::Timer::after_millis(milliseconds).await;
    }
}
