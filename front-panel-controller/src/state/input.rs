use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, signal::Signal};

pub use druzhba_common::protocol_types::{
    IfGainMode, LedState, Mode, RfGainMode, TransmitMode, Wm8940Command as Wm8940Config,
};

#[derive(Debug, Clone, Copy)]
pub struct LedUpdate {
    pub led_id: u8,
    pub state: LedState,
}

#[derive(Debug, Clone, Copy)]
pub struct MeterState {
    pub rssi_dbm: i8,
    pub forward_power_mw: u16,
    pub vswr_x100: u16,
    pub mode: Mode,
    pub transmit_mode: TransmitMode,
    pub agc_mode: IfGainMode,
    pub rf_gain_mode: RfGainMode,
    pub filter_bw_hz: u16,
}

pub type LedSignal = Signal<ThreadModeRawMutex, LedUpdate>;

pub type Wm8940Signal = Signal<ThreadModeRawMutex, Wm8940Config>;

pub type MeterStateSignal = Signal<ThreadModeRawMutex, MeterState>;

pub struct InputState {
    pub leds: LedSignal,
    pub wm8940: Wm8940Signal,
    pub displays_enabled: Signal<ThreadModeRawMutex, bool>,
    pub meter_state: MeterStateSignal,
}

impl InputState {
    pub const fn new() -> Self {
        Self {
            leds: Signal::new(),
            wm8940: Signal::new(),
            displays_enabled: Signal::new(),
            meter_state: Signal::new(),
        }
    }
}
