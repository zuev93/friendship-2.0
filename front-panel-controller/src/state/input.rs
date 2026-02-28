use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, signal::Signal};

pub use druzhba_common::protocol_types::{
    IfGainMode, LedState, Mode, RfGainMode, SweepStatus, TransmitMode,
    Wm8940Command as Wm8940Config, WATERFALL_BINS,
};

#[derive(Debug, Clone, Copy)]
pub struct LedUpdate {
    pub led_id: u8,
    pub state: LedState,
}

#[derive(Debug, Clone, Copy)]
pub struct RadioState {
    pub rssi_dbm: i8,
    pub forward_power_mw: u16,
    pub vswr_x100: u16,
    pub mode: Mode,
    pub transmit_mode: TransmitMode,
    pub agc_mode: IfGainMode,
    pub rf_gain_mode: RfGainMode,
    pub filter_bw_hz: u16,
    pub frequency: u32,
    pub band: u8,
    pub nb_enabled: bool,
    pub clarifier_mode: u8,
    pub clarifier_raw: i16,
    pub rf_power_centipercent: u16,
    pub volume_raw: i16,
    pub squelch_raw: i16,
}

pub type LedSignal = Signal<ThreadModeRawMutex, LedUpdate>;

pub type Wm8940Signal = Signal<ThreadModeRawMutex, Wm8940Config>;

pub type RadioStateSignal = Signal<ThreadModeRawMutex, RadioState>;

#[derive(Clone, Copy)]
pub struct WaterfallLineData {
    pub center_freq: u32,
    pub span_hz: u32,
    pub sweep_status: SweepStatus,
    pub bins: [i8; WATERFALL_BINS],
}

pub type WaterfallLineSignal = Signal<ThreadModeRawMutex, WaterfallLineData>;

pub struct InputState {
    pub leds: LedSignal,
    pub wm8940: Wm8940Signal,
    pub radio_state: RadioStateSignal,
    pub waterfall_line: WaterfallLineSignal,
}

impl InputState {
    pub const fn new() -> Self {
        Self {
            leds: Signal::new(),
            wm8940: Signal::new(),
            radio_state: Signal::new(),
            waterfall_line: Signal::new(),
        }
    }
}
