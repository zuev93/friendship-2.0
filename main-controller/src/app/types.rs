#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    StandBy,
    WarmUp,
    Rx,
    Tx,
}

#[derive(Clone, Copy, PartialEq)]
#[allow(dead_code)]
// TODO check actual values and consider splitting the type.
// Hardware wise we have 2 filters + 3 filters -> 6 combinations in total.
pub enum FilterType {
    None,
    Single,
    DoubleNarrow,
    DoubleWide,
}

impl FilterType {
    // TODO move to settings
    // TODO tune us in
    const FILTER_CENTER_HZ: u32 = 10_000_000; // 10 MHz center
    const WIDE_FILTER_BANDWIDTH_HZ: u32 = 2_400; // 2.4 kHz bandwidth (SSB)
    const NARROW_FILTER_BANDWIDTH_HZ: u32 = 1_200;
    const SINGLE_FILTER_BANDWIDTH_HZ: u32 = 3_600;

    pub fn center_frequency_hz(self) -> u32 {
        Self::FILTER_CENTER_HZ
    }

    pub fn bandwidth_hz(self) -> u32 {
        match self {
            Self::DoubleNarrow => Self::WIDE_FILTER_BANDWIDTH_HZ,
            Self::DoubleWide => Self::NARROW_FILTER_BANDWIDTH_HZ,
            Self::Single => Self::SINGLE_FILTER_BANDWIDTH_HZ,
            Self::None => Self::SINGLE_FILTER_BANDWIDTH_HZ,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransmitMode {
    Usb,
    Lsb,
    Cw,
    Am,
}

impl TransmitMode {
    pub fn next(self) -> Self {
        match self {
            TransmitMode::Usb => TransmitMode::Lsb,
            TransmitMode::Lsb => TransmitMode::Cw,
            TransmitMode::Cw => TransmitMode::Am,
            TransmitMode::Am => TransmitMode::Usb,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
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
#[allow(dead_code)]
pub enum RfGainMode {
    Attenuator,
    Normal,
    RfSingle, // only amp before filter
    RfDouble, // amp before filter + pre amp
}

impl RfGainMode {
    pub fn next(self) -> Self {
        match self {
            RfGainMode::Attenuator => RfGainMode::Normal,
            RfGainMode::Normal => RfGainMode::RfSingle,
            RfGainMode::RfSingle => RfGainMode::RfDouble,
            RfGainMode::RfDouble => RfGainMode::Attenuator,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IfGainMode {
    Manual,
    AgcFast,
    AgcSlow,
}

impl IfGainMode {
    pub fn toggle(self) -> Self {
        match self {
            IfGainMode::Manual => IfGainMode::AgcFast,
            IfGainMode::AgcFast => IfGainMode::AgcSlow,
            IfGainMode::AgcSlow => IfGainMode::Manual,
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

// Audio and RF control values from 16-bit ADC
pub type Volume = i16; // Audio volume (0 = mute, max ≈ +26500)
pub type Microphone = i16; // Microphone gain (0 to max ≈ +26500)
pub type IfGain = i16; // IF gain control (0 to max ≈ +26500)
pub type ClarifierValue = i16; // RIT/XIT clarifier (0 to max ≈ +26500)
pub type Squelch = i16; // Squelch level (0 to max ≈ +26500)

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

    /// Convert raw ADC value to centipercent
    /// ADC range: 0-26500 maps to 0-10000 (0.00% - 100.00%)
    pub fn from_adc_raw(raw: i16) -> Self {
        let centipercent = ((raw.max(0) as u32 * 10000) / 26500) as u16;
        Self::new(centipercent)
    }
}

pub type RfPower = RfPowerPercent;
