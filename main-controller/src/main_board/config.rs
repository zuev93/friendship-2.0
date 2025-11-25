/*
 * Transceiver Settings Persistence
 *
 * Stores and retrieves settings from EEPROM/Flash.
 * Settings survive power cycles.
 *
 * TODO: Implement actual Flash storage with Embassy
 */

use crate::app::types::{FilterType, TransmitMode};
use common::drivers::pca9534::Pin as Pca9534Pin;

const _SETTINGS_BASE_ADDR: u16 = 0;
const _SETTINGS_MAGIC: u16 = 0xDBA3; // "Druzhba-3" magic number
const _SETTINGS_VERSION: u8 = 6; // Increment version for frequency mode

// Hardware configuration for Filter Select module
pub const FILTER_SELECT_I2C_ADDR: u8 = 0x23;

/// Filter Select pin mapping on PCA9534
#[derive(Debug, Clone, Copy)]
pub struct FilterSelectPins {
    pub single_filter: Pca9534Pin,        // Pin for Single filter relay
    pub double_narrow_filter: Pca9534Pin, // Pin for DoubleNarrow filter relay
    pub double_wide_filter: Pca9534Pin,   // Pin for DoubleWide filter relay
    pub rx_enable: Pca9534Pin,            // Pin for +RX power enable
}

impl FilterSelectPins {
    pub const fn default() -> Self {
        Self {
            single_filter: Pca9534Pin::Pin0,
            double_narrow_filter: Pca9534Pin::Pin1,
            double_wide_filter: Pca9534Pin::Pin2,
            rx_enable: Pca9534Pin::Pin3,
        }
    }

    pub fn get_filter_pin(&self, filter: FilterType) -> Pca9534Pin {
        match filter {
            FilterType::Single => self.single_filter,
            FilterType::DoubleNarrow => self.double_narrow_filter,
            FilterType::DoubleWide => self.double_wide_filter,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub tx_power: u16,               // 12-bit DAC value (0-4095)
    pub transmit_mode: TransmitMode, // Operating mode (CW/USB/LSB/AM)
    pub filter: FilterType,          // IF filter selection (Narrow/Wide)
    #[allow(dead_code)]
    pub af_volume: u8, // AF amplifier volume (0-100%)
    #[allow(dead_code)]
    pub mic_gain: u8, // Microphone gain (0-100%)
    pub tone_frequency: u32,         // Tone generator frequency (Hz)
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            tx_power: 2048,                   // Mid-range (50%)
            transmit_mode: TransmitMode::Usb, // USB by default
            filter: FilterType::Single,
            af_volume: 50,       // 50% volume by default
            mic_gain: 50,        // 50% mic gain by default
            tone_frequency: 700, // 700 Hz CW sidetone by default
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        Self::default()
    }
}
