use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::{
    app::types::{ClarifierMode, ClarifierValue, FilterType, IfGainMode},
    consts::AUDIO_BUFFER_SIZE,
};

use super::types::{
    Band, CouplerMetrics, Frequency, IfGain, Microphone, Mode, RfGainMode, RfPower, Squelch,
    TransmitMode, Volume,
};

pub static CURRENT_MODE: Signal<ThreadModeRawMutex, Mode> = Signal::new();
// TODO trigger me
pub static CURRENT_FILTER: Signal<ThreadModeRawMutex, FilterType> = Signal::new();
pub static CURRENT_BAND: Signal<ThreadModeRawMutex, Band> = Signal::new();
pub static CURRENT_FREQUENCY: Signal<ThreadModeRawMutex, Frequency> = Signal::new();

// Audio and RF control signals
pub static CURRENT_VOLUME: Signal<ThreadModeRawMutex, Volume> = Signal::new();
pub static CURRENT_MICROPHONE: Signal<ThreadModeRawMutex, Microphone> = Signal::new();
pub static CURRENT_RF_POWER: Signal<ThreadModeRawMutex, RfPower> = Signal::new();
pub static CURRENT_IF_GAIN: Signal<ThreadModeRawMutex, IfGain> = Signal::new();
pub static CURRENT_IF_GAIN_MODE: Signal<ThreadModeRawMutex, IfGainMode> = Signal::new();
pub static CURRENT_CLARIFIER_MODE: Signal<ThreadModeRawMutex, ClarifierMode> = Signal::new();
pub static CURRENT_CLARIFIER_VALUE: Signal<ThreadModeRawMutex, ClarifierValue> = Signal::new();
pub static CURRENT_SQUELCH: Signal<ThreadModeRawMutex, Squelch> = Signal::new();

pub static CURRENT_TRANSMIT_MODE: Signal<ThreadModeRawMutex, TransmitMode> = Signal::new();
pub static CURRENT_RF_GAIN_MODE: Signal<ThreadModeRawMutex, RfGainMode> = Signal::new();
pub static TONE_ACTIVE: Signal<ThreadModeRawMutex, bool> = Signal::new();
pub static BUTTON_BEEP: Signal<ThreadModeRawMutex, ()> = Signal::new();

pub static COUPLER_METRICS: Signal<ThreadModeRawMutex, CouplerMetrics> = Signal::new();

pub static AUDIO_BUFFER_HEADPHONES: Signal<ThreadModeRawMutex, [u16; AUDIO_BUFFER_SIZE]> =
    Signal::new();
pub static AUDIO_BUFFER_SPEAKERS: Signal<ThreadModeRawMutex, [u16; AUDIO_BUFFER_SIZE]> =
    Signal::new();
pub static AUDIO_BUFFER_TX: Signal<ThreadModeRawMutex, [u16; AUDIO_BUFFER_SIZE]> = Signal::new();
