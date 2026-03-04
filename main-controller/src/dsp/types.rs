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
