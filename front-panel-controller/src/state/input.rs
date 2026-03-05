use druzhba_common::PlatformMutex;
use embassy_sync::signal::Signal;

use super::error_log::ErrorLog;
use super::menu::MenuScreenSignal;

pub use druzhba_common::protocol_types::{
    CrashInfoCommand, IfGainMode, LedState, Mode, RfGainMode, SweepStatus, TransmitMode,
    Wm8940Command as Wm8940Config, WATERFALL_BINS,
};

use druzhba_common::error::BsodError;

#[derive(Clone, Copy)]
pub enum FatalError {
    Init(BsodError),
    Crash(CrashInfoCommand),
}

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
    pub cursor_index: u8,
    pub cursor_editing: bool,
}

pub type LedSignal = Signal<PlatformMutex, LedUpdate>;

pub type Wm8940Signal = Signal<PlatformMutex, Wm8940Config>;

pub type RadioStateSignal = Signal<PlatformMutex, RadioState>;

#[derive(Clone, Copy)]
pub struct WaterfallLineData {
    pub center_freq: u32,
    pub span_hz: u32,
    pub sweep_status: SweepStatus,
    pub live_start: u8,
    pub live_end: u8,
    pub bins: [i8; WATERFALL_BINS],
}

pub type WaterfallLineSignal = Signal<PlatformMutex, WaterfallLineData>;

pub type FatalSignal = Signal<PlatformMutex, FatalError>;

pub struct InputState {
    pub leds: LedSignal,
    pub wm8940: Wm8940Signal,
    pub radio_state: RadioStateSignal,
    pub waterfall_line: WaterfallLineSignal,
    pub menu_screen: MenuScreenSignal,
    pub fatal: FatalSignal,
    pub error_log: ErrorLog,
}

impl InputState {
    pub const fn new() -> Self {
        Self {
            leds: Signal::new(),
            wm8940: Signal::new(),
            radio_state: Signal::new(),
            waterfall_line: Signal::new(),
            menu_screen: Signal::new(),
            fatal: Signal::new(),
            error_log: ErrorLog::new(),
        }
    }
}
