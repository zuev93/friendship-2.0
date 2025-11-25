use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, signal::Signal};

pub use druzhba_common::protocol_types::{
    LedState, Wm8940Command as Wm8940Config, DISPLAY_BUFFER_SIZE,
};

#[derive(Debug, Clone, Copy)]
pub struct LedUpdate {
    pub led_id: u8,
    pub state: LedState,
}

pub type LedSignal = Signal<ThreadModeRawMutex, LedUpdate>;

pub type SMeterSignal = Signal<ThreadModeRawMutex, u16>;

pub type Wm8940Signal = Signal<ThreadModeRawMutex, Wm8940Config>;

pub struct DisplayBuffer {
    pub buffer: [u8; DISPLAY_BUFFER_SIZE],
    pub dirty: bool,
}

pub type DisplaySignal = Signal<ThreadModeRawMutex, DisplayBuffer>;

pub struct InputState {
    pub leds: LedSignal,
    pub s_meter: SMeterSignal,
    pub wm8940: Wm8940Signal,
    pub displays: [DisplaySignal; 2],
    pub displays_enabled: Signal<ThreadModeRawMutex, bool>,
}

impl InputState {
    pub const fn new() -> Self {
        Self {
            leds: Signal::new(),
            s_meter: Signal::new(),
            wm8940: Signal::new(),
            displays: [Signal::new(), Signal::new()],
            displays_enabled: Signal::new(),
        }
    }
}
