use crate::consts::AUDIO_BUFFER_SIZE;

pub const DSP_BLOCK_SIZE: usize = AUDIO_BUFFER_SIZE;
pub const FFT_SIZE: usize = 1024;
pub const FFT_BINS: usize = FFT_SIZE / 2;
pub const NCO_CENTER_HZ: u32 = 50_000;

pub struct IqBuffer {
    pub i: [f32; DSP_BLOCK_SIZE],
    pub q: [f32; DSP_BLOCK_SIZE],
}

impl IqBuffer {
    pub const fn zero() -> Self {
        Self {
            i: [0.0; DSP_BLOCK_SIZE],
            q: [0.0; DSP_BLOCK_SIZE],
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum DemodMode {
    Usb,
    Lsb,
    Cw,
    Am,
    Fm,
    Sam,
}

impl DemodMode {
    pub fn from_index(idx: u8) -> Self {
        match idx {
            0 => Self::Usb,
            1 => Self::Lsb,
            2 => Self::Cw,
            3 => Self::Am,
            4 => Self::Fm,
            _ => Self::Sam,
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum AgcPreset {
    SsbFast,
    SsbSlow,
    Cw,
    Am,
    Manual,
    Off,
}

impl AgcPreset {
    pub fn attack_ms(self) -> f32 {
        match self {
            Self::SsbFast => 2.0,
            Self::SsbSlow => 5.0,
            Self::Cw => 1.0,
            Self::Am => 10.0,
            Self::Manual | Self::Off => 0.0,
        }
    }

    pub fn release_ms(self) -> f32 {
        match self {
            Self::SsbFast => 200.0,
            Self::SsbSlow => 500.0,
            Self::Cw => 300.0,
            Self::Am => 1000.0,
            Self::Manual | Self::Off => 0.0,
        }
    }

    pub fn hang_ms(self) -> f32 {
        match self {
            Self::SsbFast => 100.0,
            Self::SsbSlow => 300.0,
            Self::Cw => 200.0,
            Self::Am | Self::Manual | Self::Off => 0.0,
        }
    }
}

pub struct FilterPreset {
    pub bw_hz: f32,
    pub shift_hz: f32,
    pub taps: usize,
}

impl FilterPreset {
    pub const CW_NARROW: Self = Self {
        bw_hz: 200.0,
        shift_hz: 700.0,
        taps: 255,
    };
    pub const CW_WIDE: Self = Self {
        bw_hz: 500.0,
        shift_hz: 700.0,
        taps: 127,
    };
    pub const SSB: Self = Self {
        bw_hz: 2400.0,
        shift_hz: 1500.0,
        taps: 127,
    };
    pub const SSB_WIDE: Self = Self {
        bw_hz: 3100.0,
        shift_hz: 1650.0,
        taps: 95,
    };
    pub const AM: Self = Self {
        bw_hz: 6000.0,
        shift_hz: 0.0,
        taps: 63,
    };
    pub const AM_WIDE: Self = Self {
        bw_hz: 9000.0,
        shift_hz: 0.0,
        taps: 63,
    };
    pub const FM_NARROW: Self = Self {
        bw_hz: 12000.0,
        shift_hz: 0.0,
        taps: 63,
    };
    pub const FM_WIDE: Self = Self {
        bw_hz: 15000.0,
        shift_hz: 0.0,
        taps: 47,
    };

    pub fn for_mode(mode: DemodMode) -> Self {
        match mode {
            DemodMode::Usb | DemodMode::Lsb => Self::SSB,
            DemodMode::Cw => Self::CW_WIDE,
            DemodMode::Am | DemodMode::Sam => Self::AM,
            DemodMode::Fm => Self::FM_NARROW,
        }
    }

    pub fn by_index(idx: u8) -> Self {
        match idx {
            0 => Self::CW_NARROW,
            1 => Self::CW_WIDE,
            2 => Self::SSB,
            3 => Self::SSB_WIDE,
            4 => Self::AM,
            5 => Self::AM_WIDE,
            6 => Self::FM_NARROW,
            _ => Self::FM_WIDE,
        }
    }
}

pub struct FftResult {
    pub bins: [f32; FFT_BINS],
}

impl FftResult {
    pub const fn zero() -> Self {
        Self {
            bins: [0.0; FFT_BINS],
        }
    }
}
