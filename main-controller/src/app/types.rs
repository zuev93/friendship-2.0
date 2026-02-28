pub use common::protocol_types::{IfGainMode, Mode, RfGainMode, TransmitMode};

#[derive(Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum FilterType {
    None,
    Single,
    DoubleNarrow,
    DoubleWide,
}

impl FilterType {
    const FILTER_CENTER_HZ: u32 = 10_000_000;
    const WIDE_FILTER_BANDWIDTH_HZ: u32 = 2_400;
    const NARROW_FILTER_BANDWIDTH_HZ: u32 = 1_200;
    const SINGLE_FILTER_BANDWIDTH_HZ: u32 = 3_600;

    pub fn center_frequency_hz(self) -> u32 {
        Self::FILTER_CENTER_HZ
    }

    pub fn bandwidth_hz(self) -> u32 {
        match self {
            Self::DoubleNarrow => Self::NARROW_FILTER_BANDWIDTH_HZ,
            Self::DoubleWide => Self::WIDE_FILTER_BANDWIDTH_HZ,
            Self::Single => Self::SINGLE_FILTER_BANDWIDTH_HZ,
            Self::None => Self::SINGLE_FILTER_BANDWIDTH_HZ,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClarifierMode {
    Off,
    Rit,
    XIT,
}

impl ClarifierMode {
    pub fn toggle(self) -> Self {
        match self {
            ClarifierMode::Off => ClarifierMode::Rit,
            ClarifierMode::Rit => ClarifierMode::XIT,
            ClarifierMode::XIT => ClarifierMode::Off,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Band {
    Band160m,
    Band80m,
    Band40m,
    Band30m,
    Band20m,
    Band17m,
    Band15m,
    Band12m,
    Band10m,
}

impl Band {
    pub fn next(self) -> Self {
        match self {
            Band::Band160m => Band::Band80m,
            Band::Band80m => Band::Band40m,
            Band::Band40m => Band::Band30m,
            Band::Band30m => Band::Band20m,
            Band::Band20m => Band::Band17m,
            Band::Band17m => Band::Band15m,
            Band::Band15m => Band::Band12m,
            Band::Band12m => Band::Band10m,
            Band::Band10m => Band::Band10m,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Band::Band160m => Band::Band160m,
            Band::Band80m => Band::Band160m,
            Band::Band40m => Band::Band80m,
            Band::Band30m => Band::Band40m,
            Band::Band20m => Band::Band30m,
            Band::Band17m => Band::Band20m,
            Band::Band15m => Band::Band17m,
            Band::Band12m => Band::Band15m,
            Band::Band10m => Band::Band12m,
        }
    }

    pub fn lower_frequency(self) -> Frequency {
        match self {
            Band::Band160m => 1_800_000, // 1.8 MHz
            Band::Band80m => 3_500_000,  // 3.5 MHz
            Band::Band40m => 7_000_000,  // 7.0 MHz
            Band::Band30m => 10_100_000, // 10.1 MHz
            Band::Band20m => 14_000_000, // 14.0 MHz
            Band::Band17m => 18_068_000, // 18.068 MHz
            Band::Band15m => 21_000_000, // 21.0 MHz
            Band::Band12m => 24_890_000, // 24.89 MHz
            Band::Band10m => 28_000_000, // 28.0 MHz
        }
    }

    pub fn upper_frequency(self) -> Frequency {
        match self {
            Band::Band160m => 2_000_000, // 2.0 MHz
            Band::Band80m => 4_000_000,  // 4.0 MHz
            Band::Band40m => 7_300_000,  // 7.3 MHz
            Band::Band30m => 10_150_000, // 10.15 MHz
            Band::Band20m => 14_350_000, // 14.35 MHz
            Band::Band17m => 18_168_000, // 18.168 MHz
            Band::Band15m => 21_450_000, // 21.45 MHz
            Band::Band12m => 24_990_000, // 24.99 MHz
            Band::Band10m => 29_700_000, // 29.7 MHz
        }
    }
}

pub type Frequency = u32; // Frequency in Hz (0 to 4,294,967,295)

const ACCUMULATOR_MAX: i16 = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Volume(i16);

impl Volume {
    pub fn new(raw: i16) -> Self {
        Self(raw.clamp(0, ACCUMULATOR_MAX))
    }

    pub fn raw(self) -> i16 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Microphone(i16);

impl Microphone {
    pub fn new(raw: i16) -> Self {
        Self(raw.clamp(0, ACCUMULATOR_MAX))
    }

    pub fn raw(self) -> i16 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IfGain(i16);

impl IfGain {
    pub fn new(raw: i16) -> Self {
        Self(raw.clamp(0, ACCUMULATOR_MAX))
    }

    pub fn raw(self) -> i16 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClarifierValue(i16);

impl ClarifierValue {
    pub fn new(raw: i16) -> Self {
        Self(raw.clamp(0, ACCUMULATOR_MAX))
    }

    pub fn raw(self) -> i16 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Squelch(i16);

impl Squelch {
    pub fn new(raw: i16) -> Self {
        Self(raw.clamp(0, ACCUMULATOR_MAX))
    }

    pub fn raw(self) -> i16 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NbLevel(i16);

impl NbLevel {
    pub fn new(raw: i16) -> Self {
        Self(raw.clamp(0, ACCUMULATOR_MAX))
    }

    pub fn raw(self) -> i16 {
        self.0
    }
}

/// RF Power level in hundredths of percent (0.00% - 100.00%)
/// Provides intuitive control over transmitter power output with high precision
/// Range: 0-10000 represents 0.00% to 100.00%
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RfPowerPercent {
    pub centipercent: u16, // 0-10000 (0.00% - 100.00%)
}

impl RfPowerPercent {
    pub fn new(centipercent: u16) -> Self {
        Self {
            centipercent: centipercent.min(10000),
        }
    }

    pub fn from_accumulated(raw: i16) -> Self {
        let centipercent = ((raw.max(0) as u32 * 10000) / ACCUMULATOR_MAX as u32) as u16;
        Self::new(centipercent)
    }
}

pub type RfPower = RfPowerPercent;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SweepRequest {
    SetFrequency(Frequency),
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaTemperatures {
    pub driver_raw: i16,
    pub final_raw: i16,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CouplerMetrics {
    pub forward_w: f32,
    pub reflected_w: f32,
    pub vswr: f32,
}

pub const WATERFALL_BINS: usize = 128;
pub const WATERFALL_LINES: usize = 64;

#[derive(Clone, Copy)]
pub struct WaterfallLine {
    pub bins: [i16; WATERFALL_BINS],
    pub complete: bool,
}

impl WaterfallLine {
    pub const fn new() -> Self {
        Self {
            bins: [0; WATERFALL_BINS],
            complete: false,
        }
    }
}

pub struct WaterfallBuffer {
    pub lines: [WaterfallLine; WATERFALL_LINES],
    pub write_index: usize,
}

impl WaterfallBuffer {
    pub const fn new() -> Self {
        Self {
            lines: [WaterfallLine::new(); WATERFALL_LINES],
            write_index: 0,
        }
    }

    pub fn push(&mut self, line: WaterfallLine) {
        self.lines[self.write_index] = line;
        self.write_index = (self.write_index + 1) % WATERFALL_LINES;
    }
}
