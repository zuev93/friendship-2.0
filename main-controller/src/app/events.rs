use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::consts::AUDIO_BUFFER_SIZE;

use super::types::{
    Band, ClarifierMode, ClarifierValue, CouplerMetrics, FilterType, Frequency, IfGain,
    IfGainMode, Microphone, Mode, NbLevel, PaTemperatures, RfGainMode, RfPower, Squelch,
    SweepRequest, TransmitMode, Volume, WaterfallLine,
};

pub static MODE: Signal<ThreadModeRawMutex, Mode> = Signal::new();
pub static FILTER: Signal<ThreadModeRawMutex, FilterType> = Signal::new();
pub static BAND: Signal<ThreadModeRawMutex, Band> = Signal::new();
pub static FREQUENCY: Signal<ThreadModeRawMutex, Frequency> = Signal::new();
pub static SWEEP_REQUEST: Signal<ThreadModeRawMutex, SweepRequest> = Signal::new();
pub static SWEEP_ACTIVE: Signal<ThreadModeRawMutex, bool> = Signal::new();
pub static RSSI_FAST_MODE: Signal<ThreadModeRawMutex, bool> = Signal::new();
pub static IF_GAIN_MODE: Signal<ThreadModeRawMutex, IfGainMode> = Signal::new();
pub static WATERFALL_LINE: Signal<ThreadModeRawMutex, WaterfallLine> = Signal::new();

pub static VOLUME: Signal<ThreadModeRawMutex, Volume> = Signal::new();
pub static MICROPHONE: Signal<ThreadModeRawMutex, Microphone> = Signal::new();
pub static RF_POWER: Signal<ThreadModeRawMutex, RfPower> = Signal::new();
pub static IF_GAIN: Signal<ThreadModeRawMutex, IfGain> = Signal::new();
pub static CLARIFIER_MODE: Signal<ThreadModeRawMutex, ClarifierMode> = Signal::new();
pub static CLARIFIER_VALUE: Signal<ThreadModeRawMutex, ClarifierValue> = Signal::new();
pub static SQUELCH: Signal<ThreadModeRawMutex, Squelch> = Signal::new();
pub static NB_ENABLED: Signal<ThreadModeRawMutex, bool> = Signal::new();
pub static NB_LEVEL: Signal<ThreadModeRawMutex, NbLevel> = Signal::new();
pub static NB_ACTIVE: Signal<ThreadModeRawMutex, bool> = Signal::new();

pub static TX_THERMAL_CONSTRAINT: Signal<ThreadModeRawMutex, i32> = Signal::new();
pub static TRANSMIT_MODE: Signal<ThreadModeRawMutex, TransmitMode> = Signal::new();
pub static RF_GAIN_MODE: Signal<ThreadModeRawMutex, RfGainMode> = Signal::new();
pub static TONE: Signal<ThreadModeRawMutex, bool> = Signal::new();
pub static BUTTON_BEEP: Signal<ThreadModeRawMutex, ()> = Signal::new();

pub static COUPLER_METRICS: Signal<ThreadModeRawMutex, CouplerMetrics> = Signal::new();
pub static PA_TEMPERATURES: Signal<ThreadModeRawMutex, PaTemperatures> = Signal::new();

pub static AUDIO_BUFFER_HEADPHONES: Signal<ThreadModeRawMutex, [u16; AUDIO_BUFFER_SIZE]> =
    Signal::new();
pub static AUDIO_BUFFER_SPEAKERS: Signal<ThreadModeRawMutex, [u16; AUDIO_BUFFER_SIZE]> =
    Signal::new();
pub static AUDIO_BUFFER_TX: Signal<ThreadModeRawMutex, [u16; AUDIO_BUFFER_SIZE]> = Signal::new();
