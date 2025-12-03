use common::drivers::ads1115::{ADS1115Config, ADS1115};
use common::drivers::pca9534::{Pin, PCA9534};
use embassy_time::Instant;

use crate::app::types::{FilterType, IfGainMode, Mode};
use crate::i2c_map::I2cAddress;
use crate::main_board::types::{MainBoardI2C, MainBoardI2CMutex, RssiDbm};
use common::drivers::mcp4725::MCP4725;

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

const IO_F1_PIN: Pin = Pin::Pin0;
const IO_F2_PIN: Pin = Pin::Pin1;
const IO_F3_PIN: Pin = Pin::Pin2;
const IO_RX_PIN: Pin = Pin::Pin3;

#[derive(Debug, Clone, Copy)]
pub struct RssiData {
    pub rssi1: RssiDbm, // AIN0 - Primary RSSI
    pub rssi2: RssiDbm, // AIN1 - Secondary RSSI
}

pub struct IfAmplifier {
    dac_gain: MCP4725<MainBoardI2C>,
    adc_rssi: ADS1115<MainBoardI2C>,
    io: PCA9534<MainBoardI2C>,
    rssi_dbm: RssiDbm,
    desired_manual_gain: i16,
    current_agc_gain: i16,    // Current AGC gain for smoothing
    last_agc_update: Instant, // Last AGC update time
    if_gain_mode: IfGainMode,
    mode: Mode,
    filter_type: FilterType,
}

impl IfAmplifier {
    pub fn new(
        i2c: &'static MainBoardI2CMutex,
        dac_addr: I2cAddress,
        adc_addr: I2cAddress,
        io_addr: I2cAddress,
    ) -> Self {
        Self {
            dac_gain: MCP4725::new(dac_addr.into(), i2c),
            adc_rssi: ADS1115::new(adc_addr.into(), ADS1115Config::default(), i2c),
            io: PCA9534::new(io_addr.into(), i2c),
            if_gain_mode: IfGainMode::Manual,
            rssi_dbm: RssiDbm { dbm: 0 },
            desired_manual_gain: 0,
            current_agc_gain: MAX_GAIN / 2, // Start at mid-gain
            last_agc_update: Instant::now(),
            mode: Mode::StandBy,
            filter_type: FilterType::None,
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
        if self.mode == Mode::WarmUp {
            self.init().await?;
        } else if self.mode == Mode::StandBy {
            return Ok(());
        }
        let mut port: u8 = 0;

        if self.mode == Mode::Rx {
            port |= IO_RX_PIN.mask();
        }
        match self.filter_type {
            FilterType::None | FilterType::Single => {
                port |= IO_F1_PIN.mask();
            }
            FilterType::DoubleNarrow => {
                port |= IO_F2_PIN.mask();
            }
            FilterType::DoubleWide => {
                port |= IO_F3_PIN.mask();
            }
        }

        self.io
            .write_port(port)
            .await
            .map_err(|_| "Failed to write if IO")?;

        match self.mode {
            Mode::Rx => {
                let value = match self.if_gain_mode {
                    IfGainMode::Manual => self.desired_manual_gain,
                    IfGainMode::AgcFast => self.calculate_agc_gain(false),
                    IfGainMode::AgcSlow => self.calculate_agc_gain(true),
                };

                let value = value.clamp(MIN_GAIN, MAX_GAIN);
                let dac_value = ((value as u32 * 4095) / 26500) as u16;

                self.dac_gain
                    .set_raw(dac_value)
                    .await
                    .map_err(|_| "Failed to set IF gain")
            }
            Mode::Tx => self
                .dac_gain
                .write_eeprom_power_down()
                .await
                .map_err(|_| "Failed to write EEPROM power down"),
            Mode::StandBy => Ok(()),
            Mode::WarmUp => self.init().await,
        }
    }

    pub async fn read_rssi(&mut self) -> Result<RssiData, &'static str> {
        async {
            let rssi1 = self
                .adc_rssi
                .read_ain0()
                .await
                .map_err(|_| "Failed to read RSSI1")?;
            let rssi2 = self
                .adc_rssi
                .read_ain1()
                .await
                .map_err(|_| "Failed to read RSSI2")?;
            Ok::<RssiData, &'static str>(RssiData {
                rssi1: RssiDbm::from_adc_raw(rssi1),
                rssi2: RssiDbm::from_adc_raw(rssi2),
            })
        }
        .await
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

    async fn init(&mut self) -> Result<(), &'static str> {
        self.adc_rssi
            .init()
            .await
            .map_err(|_| "Failed to init rssi reader")?;
        self.io
            .init()
            .await
            .map_err(|_| "Faield to init IO extender for if gain control")?;
        self.io
            .set_direction(0x0)
            .await
            .map_err(|_| "Faield to init IO extender for if gain control")?;
        Ok(())
    }
}
