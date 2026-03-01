use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::watch::Watch;

use crate::consts::AUDIO_BUFFER_SIZE;

use super::types::{
    Band, ClarifierMode, ClarifierValue, Compression, CouplerMetrics, FilterType, Frequency,
    IfGain, IfGainMode, Microphone, Mode, NbLevel, PaTemperatures, RfGainMode, RfPower, Squelch,
    SweepRequest, TransmitMode, Volume, WaterfallLine,
};

pub static MODE: Watch<ThreadModeRawMutex, Mode, 16> = Watch::new();
pub static FILTER: Watch<ThreadModeRawMutex, FilterType, 4> = Watch::new();
pub static BAND: Watch<ThreadModeRawMutex, Band, 2> = Watch::new();
pub static FREQUENCY: Watch<ThreadModeRawMutex, Frequency, 6> = Watch::new();
pub static SWEEP_REQUEST: Watch<ThreadModeRawMutex, SweepRequest, 2> = Watch::new();
pub static SWEEP_ACTIVE: Watch<ThreadModeRawMutex, bool, 2> = Watch::new();
pub static RSSI_FAST_MODE: Watch<ThreadModeRawMutex, bool, 2> = Watch::new();
pub static IF_GAIN_MODE: Watch<ThreadModeRawMutex, IfGainMode, 4> = Watch::new();
pub static WATERFALL_LINE: Watch<ThreadModeRawMutex, WaterfallLine, 2> = Watch::new();

pub static VOLUME: Watch<ThreadModeRawMutex, Volume, 4> = Watch::new();
pub static MICROPHONE: Watch<ThreadModeRawMutex, Microphone, 2> = Watch::new();
pub static RF_POWER: Watch<ThreadModeRawMutex, RfPower, 4> = Watch::new();
pub static IF_GAIN: Watch<ThreadModeRawMutex, IfGain, 2> = Watch::new();
pub static CLARIFIER_MODE: Watch<ThreadModeRawMutex, ClarifierMode, 4> = Watch::new();
pub static CLARIFIER_VALUE: Watch<ThreadModeRawMutex, ClarifierValue, 2> = Watch::new();
pub static SQUELCH: Watch<ThreadModeRawMutex, Squelch, 4> = Watch::new();
pub static SQUELCH_ENABLED: Watch<ThreadModeRawMutex, bool, 2> = Watch::new();
pub static NB_ENABLED: Watch<ThreadModeRawMutex, bool, 2> = Watch::new();
pub static NB_LEVEL: Watch<ThreadModeRawMutex, NbLevel, 2> = Watch::new();
pub static NB_ACTIVE: Watch<ThreadModeRawMutex, bool, 2> = Watch::new();

pub static TX_THERMAL_CONSTRAINT: Watch<ThreadModeRawMutex, i32, 2> = Watch::new();
pub static TRANSMIT_MODE: Watch<ThreadModeRawMutex, TransmitMode, 4> = Watch::new();
pub static RF_GAIN_MODE: Watch<ThreadModeRawMutex, RfGainMode, 4> = Watch::new();
pub static TONE: Watch<ThreadModeRawMutex, bool, 2> = Watch::new();
pub static BUTTON_BEEP: Watch<ThreadModeRawMutex, (), 2> = Watch::new();

pub static ALC_CONSTRAINT: Watch<ThreadModeRawMutex, i32, 2> = Watch::new();
pub static COUPLER_METRICS: Watch<ThreadModeRawMutex, CouplerMetrics, 2> = Watch::new();
pub static PA_TEMPERATURES: Watch<ThreadModeRawMutex, PaTemperatures, 2> = Watch::new();

pub static COMPRESSION: Watch<ThreadModeRawMutex, Compression, 2> = Watch::new();
pub static COMPRESSION_METER: Watch<ThreadModeRawMutex, u8, 2> = Watch::new();

pub static AUDIO_BUFFER_HEADPHONES: Watch<ThreadModeRawMutex, [u16; AUDIO_BUFFER_SIZE], 2> =
    Watch::new();
pub static AUDIO_BUFFER_SPEAKERS: Watch<ThreadModeRawMutex, [u16; AUDIO_BUFFER_SIZE], 2> =
    Watch::new();
pub static AUDIO_BUFFER_TX: Watch<ThreadModeRawMutex, [u16; AUDIO_BUFFER_SIZE], 2> = Watch::new();
