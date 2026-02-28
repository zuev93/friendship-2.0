use embassy_time::Duration;

pub const QEI_POLL_INTERVAL: Duration = Duration::from_millis(2);
pub const BUTTON_POLL_INTERVAL: Duration = Duration::from_millis(5);

pub const TX_QUEUE_SIZE: usize = 16;
pub const OUTPUT_EVENTS_QUEUE_SIZE: usize = 32;
