#![no_std]

pub mod consts;

#[cfg(feature = "target")]
#[path = "app/cordic_math.rs"]
pub mod cordic_math;

#[cfg(not(feature = "target"))]
#[path = "cordic_math_soft.rs"]
pub mod cordic_math;

#[cfg(feature = "target")]
#[path = "app/fmac_fir.rs"]
pub mod fmac_fir;

#[cfg(not(feature = "target"))]
#[path = "fmac_fir_soft.rs"]
pub mod fmac_fir;

pub mod dsp;

#[path = "app/tone_generator.rs"]
pub mod tone_generator;

#[cfg(not(feature = "target"))]
pub mod mixer_types;

#[cfg(not(feature = "target"))]
#[path = "app/spectral_nr.rs"]
pub mod spectral_nr;

#[cfg(not(feature = "target"))]
#[path = "app/vox.rs"]
pub mod vox;

#[cfg(not(feature = "target"))]
#[path = "app/audio_mixer.rs"]
pub mod audio_mixer;
