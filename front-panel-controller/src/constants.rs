use embassy_time::Duration;

pub const BUTTON_DEBOUNCE_DURATION: Duration = Duration::from_millis(20);
pub const BUTTON_MAX_DEBOUNCE_RETRIES: u32 = 10;

pub const HEADPHONES_POLL_INTERVAL: Duration = Duration::from_millis(100);

pub const POTENTIOMETERS_POLL_INTERVAL: Duration = Duration::from_millis(5);

pub const TX_QUEUE_SIZE: usize = 16;
pub const OUTPUT_EVENTS_QUEUE_SIZE: usize = 32;
