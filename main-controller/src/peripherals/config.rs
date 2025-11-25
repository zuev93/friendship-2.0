use crate::app::types::Frequency;
use common::drivers::pca9534::Pin as Pca9534Pin;
use common::drivers::tca9555::{Pin, Port};

/// LPF (Low Pass Filter) configuration for a single filter
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct LpfFilterConfig {
    pub freq_min: Frequency, // Lower frequency bound in Hz
    pub freq_max: Frequency, // Upper frequency bound in Hz
    pub pin: Pca9534Pin,     // PCA9534 pin to activate this filter
}

/// LPF control pins configuration
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct LpfControlPins {
    pub tx_pin: Pca9534Pin, // TX bypass pin (PCA9534 has only one port)
}

/// LPF complete configuration
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct LpfConfig {
    pub filters: [LpfFilterConfig; 6],
    pub control: LpfControlPins,
}

impl LpfConfig {
    pub fn default() -> Self {
        Self {
            filters: [
                LpfFilterConfig {
                    freq_min: 1_800_000,
                    freq_max: 4_000_000,
                    pin: Pca9534Pin::Pin0,
                },
                LpfFilterConfig {
                    freq_min: 5_000_000,
                    freq_max: 7_500_000,
                    pin: Pca9534Pin::Pin1,
                },
                LpfFilterConfig {
                    freq_min: 10_000_000,
                    freq_max: 15_000_000,
                    pin: Pca9534Pin::Pin2,
                },
                LpfFilterConfig {
                    freq_min: 18_000_000,
                    freq_max: 22_000_000,
                    pin: Pca9534Pin::Pin3,
                },
                LpfFilterConfig {
                    freq_min: 24_000_000,
                    freq_max: 30_000_000,
                    pin: Pca9534Pin::Pin4,
                },
                LpfFilterConfig {
                    freq_min: 50_000_000,
                    freq_max: 54_000_000,
                    pin: Pca9534Pin::Pin5,
                },
            ],
            control: LpfControlPins {
                tx_pin: Pca9534Pin::Pin6, // TX bypass relay
            },
        }
    }

    /// Find the appropriate filter for a given frequency
    /// Returns the pin for exact match, or the closest filter if no exact match
    #[allow(dead_code)]
    pub fn find_filter(&self, frequency: Frequency) -> Pca9534Pin {
        let mut closest_filter = &self.filters[0];
        let mut min_distance = u32::MAX;

        for filter in &self.filters {
            let center = (filter.freq_min + filter.freq_max) / 2;
            let distance = if frequency > center {
                frequency - center
            } else {
                center - frequency
            };

            if distance < min_distance {
                min_distance = distance;
                closest_filter = filter;
            }
        }

        closest_filter.pin
    }
}

/// BPF (Band Pass Filter) configuration for a single filter
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct BpfFilterConfig {
    pub freq_min: Frequency, // Lower frequency bound in Hz
    pub freq_max: Frequency, // Upper frequency bound in Hz
    pub port: Port,          // TCA9555 port
    pub pin: Pin,            // TCA9555 pin to activate this filter
}

/// BPF control pins configuration
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct BpfControlPins {
    pub att_port: Port,    // Attenuator port
    pub att_pin: Pin,      // Attenuator pin
    pub rf_amp_port: Port, // RF amplifier port
    pub rf_amp_pin: Pin,   // RF amplifier pin
    pub tx_port: Port,     // TX bypass port
    pub tx_pin: Pin,       // TX bypass pin
}

/// BPF complete configuration
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BpfConfig {
    pub filters: [BpfFilterConfig; 9],
    pub control: BpfControlPins,
}

impl BpfConfig {
    pub fn default() -> Self {
        Self {
            filters: [
                // Port0: Filter pins 0-7
                // 160m band (1.8 - 2.0 MHz)
                BpfFilterConfig {
                    freq_min: 1_800_000,
                    freq_max: 2_000_000,
                    port: Port::Port0,
                    pin: Pin::Pin0,
                },
                // 80m band (3.5 - 4.0 MHz)
                BpfFilterConfig {
                    freq_min: 3_500_000,
                    freq_max: 4_000_000,
                    port: Port::Port0,
                    pin: Pin::Pin1,
                },
                // 60m band (5.3 - 5.4 MHz)
                BpfFilterConfig {
                    freq_min: 5_300_000,
                    freq_max: 5_400_000,
                    port: Port::Port0,
                    pin: Pin::Pin2,
                },
                // 40m band (7.0 - 7.3 MHz)
                BpfFilterConfig {
                    freq_min: 7_000_000,
                    freq_max: 7_300_000,
                    port: Port::Port0,
                    pin: Pin::Pin3,
                },
                // 30m band (10.1 - 10.15 MHz)
                BpfFilterConfig {
                    freq_min: 10_100_000,
                    freq_max: 10_150_000,
                    port: Port::Port0,
                    pin: Pin::Pin4,
                },
                // 20m band (14.0 - 14.35 MHz)
                BpfFilterConfig {
                    freq_min: 14_000_000,
                    freq_max: 14_350_000,
                    port: Port::Port0,
                    pin: Pin::Pin5,
                },
                // 17m band (18.068 - 18.168 MHz)
                BpfFilterConfig {
                    freq_min: 18_068_000,
                    freq_max: 18_168_000,
                    port: Port::Port0,
                    pin: Pin::Pin6,
                },
                // 15m band (21.0 - 21.45 MHz)
                BpfFilterConfig {
                    freq_min: 21_000_000,
                    freq_max: 21_450_000,
                    port: Port::Port0,
                    pin: Pin::Pin7,
                },
                // Port1: Last filter + control pins
                // 12m band (24.89 - 24.99 MHz)
                BpfFilterConfig {
                    freq_min: 24_890_000,
                    freq_max: 24_990_000,
                    port: Port::Port1,
                    pin: Pin::Pin0,
                },
            ],
            control: BpfControlPins {
                att_port: Port::Port1,
                att_pin: Pin::Pin1, // Attenuator control
                rf_amp_port: Port::Port1,
                rf_amp_pin: Pin::Pin2, // RF amplifier enable
                tx_port: Port::Port1,
                tx_pin: Pin::Pin3, // TX bypass relay
            },
        }
    }

    pub fn find_filter(&self, frequency: Frequency) -> (Port, Pin) {
        let mut closest_filter = &self.filters[0];
        let mut min_distance = u32::MAX;

        for filter in &self.filters {
            let center = (filter.freq_min + filter.freq_max) / 2;
            let distance = if frequency > center {
                frequency - center
            } else {
                center - frequency
            };

            if distance < min_distance {
                min_distance = distance;
                closest_filter = filter;
            }
        }

        (closest_filter.port, closest_filter.pin)
    }
}

/// Peripherals subsystem configuration
#[derive(Debug, Clone)]
pub struct Settings {
    #[allow(dead_code)]
    pub lpf_config: LpfConfig,
    pub bpf_config: BpfConfig,
}

impl Settings {
    #[allow(dead_code)]
    pub fn default() -> Self {
        Self {
            lpf_config: LpfConfig::default(),
            bpf_config: BpfConfig::default(),
        }
    }
}
