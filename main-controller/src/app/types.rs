pub use common::protocol_types::{IfGainMode, Mode, RfGainMode, TransmitMode};

#[derive(Clone, Copy)]
pub struct CpuPercent(u8);

impl CpuPercent {
    pub fn new(pct: u8) -> Self {
        Self(pct)
    }

    pub fn raw(self) -> u8 {
        self.0
    }
}

#[derive(Clone, Copy)]
pub struct DisplayFps([u16; 3]);

impl DisplayFps {
    pub fn new(fps: [u16; 3]) -> Self {
        Self(fps)
    }

    pub fn raw(&self) -> &[u16; 3] {
        &self.0
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum FilterType {
    Wide,
    Medium,
    Narrow,
    CwNarrow,
}

impl FilterType {
    const FILTER_CENTER_HZ: u32 = 10_700_000;

    pub fn center_frequency_hz(self) -> u32 {
        Self::FILTER_CENTER_HZ
    }

    pub fn bandwidth_hz(self) -> u32 {
        match self {
            Self::Wide => 6_000,
            Self::Medium => 3_000,
            Self::Narrow => 2_400,
            Self::CwNarrow => 1_000,
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Wide => Self::Medium,
            Self::Medium => Self::Narrow,
            Self::Narrow => Self::CwNarrow,
            Self::CwNarrow => Self::Wide,
        }
    }

    pub fn default_for_mode(mode: TransmitMode) -> Self {
        match mode {
            TransmitMode::Usb | TransmitMode::Lsb => Self::Narrow,
            TransmitMode::Cw => Self::CwNarrow,
            TransmitMode::Am => Self::Wide,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NrLevel(i16);

impl NrLevel {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoxGain(i16);

impl VoxGain {
    pub fn new(raw: i16) -> Self {
        Self(raw.clamp(0, ACCUMULATOR_MAX))
    }

    pub fn raw(self) -> i16 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoxDelay(i16);

impl VoxDelay {
    pub fn new(raw: i16) -> Self {
        Self(raw.clamp(100, 2000))
    }

    pub fn raw(self) -> i16 {
        self.0
    }

    pub fn ms(self) -> u16 {
        self.0 as u16
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoxAntiTrip(i16);

impl VoxAntiTrip {
    pub fn new(raw: i16) -> Self {
        Self(raw.clamp(0, ACCUMULATOR_MAX))
    }

    pub fn raw(self) -> i16 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanStep(u16);

impl ScanStep {
    pub fn new(raw: u16) -> Self {
        Self(raw.clamp(100, 10000))
    }

    pub fn hz(self) -> u32 {
        self.0 as u32
    }

    pub fn add(self, delta: i16) -> Self {
        let val = (self.0 as i32 + delta as i32).clamp(100, 10000) as u16;
        Self(val)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanResume(u8);

impl ScanResume {
    pub fn new(raw: u8) -> Self {
        Self(raw.clamp(1, 10))
    }

    pub fn raw(self) -> u8 {
        self.0
    }

    pub fn secs(self) -> u64 {
        self.0 as u64
    }
}

pub type RfPower = RfPowerPercent;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioAgcMode {
    Off,
    Slow,
    Med,
    Fast,
}

impl AudioAgcMode {
    pub fn next(self) -> Self {
        match self {
            Self::Off => Self::Slow,
            Self::Slow => Self::Med,
            Self::Med => Self::Fast,
            Self::Fast => Self::Off,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DspBandwidth(u16);

impl DspBandwidth {
    pub fn new(raw: u16) -> Self {
        Self(raw.clamp(100, 3600))
    }

    pub fn raw(self) -> u16 {
        self.0
    }

    pub fn add(self, delta: i16) -> Self {
        let val = (self.0 as i32 + delta as i32).clamp(100, 3600) as u16;
        Self(val)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DspShift(i16);

impl DspShift {
    pub fn new(raw: i16) -> Self {
        Self(raw.clamp(-1000, 1000))
    }

    pub fn raw(self) -> i16 {
        self.0
    }

    pub fn add(self, delta: i16) -> Self {
        let val = (self.0 as i32 + delta as i32).clamp(-1000, 1000) as i16;
        Self(val)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CwPeakWidth(u16);

impl CwPeakWidth {
    pub fn new(raw: u16) -> Self {
        Self(raw.clamp(50, 500))
    }

    pub fn raw(self) -> u16 {
        self.0
    }

    pub fn add(self, delta: i16) -> Self {
        let val = (self.0 as i32 + delta as i32).clamp(50, 500) as u16;
        Self(val)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CwPitch(u16);

impl CwPitch {
    pub fn new(raw: u16) -> Self {
        Self(raw.clamp(400, 1000))
    }

    pub fn raw(self) -> u16 {
        self.0
    }

    pub fn add(self, delta: i16) -> Self {
        let val = (self.0 as i32 + delta as i32).clamp(400, 1000) as u16;
        Self(val)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EqGain(i8);

impl EqGain {
    pub fn new(raw: i8) -> Self {
        Self(raw.clamp(-12, 12))
    }

    pub fn raw(self) -> i8 {
        self.0
    }

    pub fn add(self, delta: i8) -> Self {
        let val = (self.0 as i16 + delta as i16).clamp(-12, 12) as i8;
        Self(val)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Compression(i16);

impl Compression {
    pub fn new(raw: i16) -> Self {
        Self(raw.clamp(0, ACCUMULATOR_MAX))
    }

    pub fn raw(self) -> i16 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SweepRequest {
    SetFrequency(Frequency),
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaTemperatures {
    pub driver_c: i16,
    pub final_c: i16,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CouplerMetrics {
    pub forward_w: f32,
    pub reflected_w: f32,
    pub vswr: f32,
}

pub const WATERFALL_BINS: usize = 240;
pub const WATERFALL_LINES: usize = 90;

#[derive(Clone, Copy)]
pub struct WaterfallLine {
    pub bins: [i8; WATERFALL_BINS],
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CwKeyMode {
    Straight,
    IambicA,
    IambicB,
}

impl CwKeyMode {
    pub fn next(self) -> Self {
        match self {
            Self::Straight => Self::IambicA,
            Self::IambicA => Self::IambicB,
            Self::IambicB => Self::Straight,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CwWpm(u8);

impl CwWpm {
    pub fn new(raw: u8) -> Self {
        Self(raw.clamp(5, 60))
    }

    pub fn raw(self) -> u8 {
        self.0
    }

    pub fn add(self, delta: i8) -> Self {
        let val = (self.0 as i16 + delta as i16).clamp(5, 60) as u8;
        Self(val)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CwWeight(u8);

impl CwWeight {
    pub fn new(raw: u8) -> Self {
        Self(raw.clamp(25, 75))
    }

    pub fn raw(self) -> u8 {
        self.0
    }

    pub fn add(self, delta: i8) -> Self {
        let val = (self.0 as i16 + delta as i16).clamp(25, 75) as u8;
        Self(val)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CwBreakInDelay(u16);

impl CwBreakInDelay {
    pub fn new(raw: u16) -> Self {
        Self(raw.clamp(100, 2000))
    }

    pub fn ms(self) -> u16 {
        self.0
    }

    pub fn add(self, delta: i16) -> Self {
        let val = (self.0 as i32 + delta as i32).clamp(100, 2000) as u16;
        Self(val)
    }
}
