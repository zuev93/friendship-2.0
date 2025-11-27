use crate::app::types::{FilterType, TransmitMode};

#[derive(Clone, Copy)]
pub struct Settings {
    pub transmit_mode: TransmitMode, // Operating mode (CW/USB/LSB/AM)
    pub filter: FilterType,          // IF filter selection (Narrow/Wide)
    #[allow(dead_code)]
    pub af_volume: u8, // AF amplifier volume (0-100%)
    #[allow(dead_code)]
    pub mic_gain: u8, // Microphone gain (0-100%)
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            transmit_mode: TransmitMode::Usb, // USB by default
            filter: FilterType::Single,
            af_volume: 50, // 50% volume by default
            mic_gain: 50,  // 50% mic gain by default
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        Self::default()
    }
}
