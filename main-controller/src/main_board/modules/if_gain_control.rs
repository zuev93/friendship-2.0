/*
 * IF Gain Control Module
 *
 * Manages IF stage gain through AD8367 VGA (Variable Gain Amplifier)
 * controlled by MCP4725 DAC.
 *
 * Controls AGC (Automatic Gain Control) for the receiver path.
 */

use embassy_time::Instant;

use crate::app::types::{IfGainMode, Mode};
use crate::i2c_map;
use crate::main_board::types::{MainBoardI2C, MainBoardI2CMutex, RssiDbm};
use common::drivers::mcp4725::MCP4725;

const DAC_ADDRESS: u8 = i2c_map::MCP4725_IF_GAIN_ADDR;

// TODO move to settings
// AGC constants
const TARGET_RSSI_DBM: i8 = -73; // S9 level target (~S9)
const MIN_GAIN: i16 = 0;
const MAX_GAIN: i16 = 26500;

// AGC time constants in milliseconds
const AGC_FAST_TIME_CONSTANT_MS: u32 = 200; // 200ms to settle (for CW, fast signals)
const AGC_SLOW_TIME_CONSTANT_MS: u32 = 2000; // 2s to settle (for SSB, AM)

// Gain adjustment factor: 1 dB change ≈ 250 gain units
const DB_TO_GAIN_FACTOR: i32 = 250;

pub struct IfGainControl {
    dac: MCP4725<MainBoardI2C>,
    rssi_dbm: RssiDbm,
    desired_manual_gain: i16,
    current_agc_gain: i16,    // Current AGC gain for smoothing
    last_agc_update: Instant, // Last AGC update time
    if_gain_mode: IfGainMode,
    mode: Mode,
}

impl IfGainControl {
    pub fn new(i2c: &'static MainBoardI2CMutex) -> Self {
        let dac = MCP4725::new(DAC_ADDRESS, i2c);
        Self {
            dac,
            if_gain_mode: IfGainMode::Manual,
            rssi_dbm: RssiDbm { dbm: 0 },
            desired_manual_gain: 0,
            current_agc_gain: MAX_GAIN / 2, // Start at mid-gain
            last_agc_update: Instant::now(),
            mode: Mode::StandBy,
        }
    }

    pub async fn set_manual_gain_raw(&mut self, raw_value: i16) -> Result<(), &'static str> {
        self.desired_manual_gain = raw_value;
        self.update_state().await
    }

    pub async fn set_if_gain_mode(&mut self, if_gain_mode: IfGainMode) -> Result<(), &'static str> {
        self.if_gain_mode = if_gain_mode;
        self.update_state().await
    }

    pub async fn set_mode(&mut self, mode: Mode) -> Result<(), &'static str> {
        self.mode = mode;
        self.update_state().await
    }

    pub async fn update_agc(&mut self, rssi: RssiDbm) -> Result<(), &'static str> {
        self.rssi_dbm = rssi;
        self.update_state().await
    }

    async fn update_state(&mut self) -> Result<(), &'static str> {
        match self.mode {
            Mode::Rx => {
                let value = match self.if_gain_mode {
                    IfGainMode::Manual => self.desired_manual_gain,
                    IfGainMode::AgcFast => self.calculate_agc_gain(false),
                    IfGainMode::AgcSlow => self.calculate_agc_gain(true),
                };

                let value = value.clamp(MIN_GAIN, MAX_GAIN);
                let dac_value = ((value as u32 * 4095) / 26500) as u16;

                self.dac
                    .set_raw(dac_value)
                    .await
                    .map_err(|_| "Failed to set IF gain")
            }
            Mode::Tx | Mode::StandBy | Mode::WarmUp => self
                .dac
                .write_eeprom_power_down()
                .await
                .map_err(|_| "Failed to write EEPROM power down"),
        }
    }

    /// Calculate AGC gain based on RSSI with time-based exponential smoothing
    ///
    /// Time-based AGC Algorithm:
    /// 1. Measure time since last update (dt)
    /// 2. Calculate target gain based on RSSI error
    /// 3. Apply exponential smoothing with time constant:
    ///    - Fast: 200ms tau (for CW, digital modes)
    ///    - Slow: 2000ms tau (for SSB, AM - smooth audio)
    ///
    /// This ensures consistent AGC behavior regardless of update frequency.
    fn calculate_agc_gain(&mut self, is_slow: bool) -> i16 {
        let now = Instant::now();
        let dt_ms = (now - self.last_agc_update).as_millis() as u32;
        self.last_agc_update = now;

        if dt_ms == 0 {
            return self.current_agc_gain;
        }

        let current_rssi = self.rssi_dbm.dbm as i32;
        let error_db = current_rssi - TARGET_RSSI_DBM as i32;

        let gain_adjustment = -error_db * DB_TO_GAIN_FACTOR;
        let target_gain = (self.current_agc_gain as i32 + gain_adjustment)
            .clamp(MIN_GAIN as i32, MAX_GAIN as i32);

        let tau_ms = if is_slow {
            AGC_SLOW_TIME_CONSTANT_MS
        } else {
            AGC_FAST_TIME_CONSTANT_MS
        };

        const SCALE: u32 = 1024;
        let alpha_scaled = ((dt_ms * SCALE) / tau_ms).min(SCALE);

        let delta = target_gain - self.current_agc_gain as i32;
        let adjustment = (delta * alpha_scaled as i32) / SCALE as i32;
        let smoothed_gain = self.current_agc_gain as i32 + adjustment;

        self.current_agc_gain = smoothed_gain.clamp(MIN_GAIN as i32, MAX_GAIN as i32) as i16;

        self.current_agc_gain
    }
}
