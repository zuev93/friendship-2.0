use common::drivers::ads1015::ADS1015;
use embassy_time::Instant;

use crate::app::types::{IfGain, IfGainMode, Mode, RfPowerPercent};
use crate::control_board::events::{PdContract, PowerTelemetry};
use crate::i2c_map::I2cAddress;
use crate::main_board::types::{MainBoardI2C, MainBoardI2CMutex, RssiDbm};
use common::drivers::mcp4728::{Channel, MCP4728};

const DAC_MAX: u16 = 4095;
const CENTIPERCENT_MAX: u32 = 10000;

const TARGET_RSSI_DBM: i8 = -73;
const MIN_GAIN: i16 = 0;
const MAX_GAIN: i16 = DAC_MAX as i16;

const AGC_FAST_TIME_CONSTANT_MS: u32 = 200;
const AGC_SLOW_TIME_CONSTANT_MS: u32 = 2000;
const DB_TO_GAIN_FACTOR: i32 = 250;

const GAIN_TOTAL_RANGE: u32 = (DAC_MAX as u32) * 2;

const CH_AGC2: Channel = Channel::A;
const CH_AGC1: Channel = Channel::B;
const CH_RX: Channel = Channel::D;

#[derive(Debug, Clone, Copy)]
pub struct RssiData {
    pub rssi1: RssiDbm,
    pub rssi2: RssiDbm,
}

pub struct IfAmplifier {
    dac: MCP4728<MainBoardI2C>,
    adc_rssi: ADS1015<MainBoardI2C>,
    rssi_dbm: RssiDbm,
    desired_manual_gain: i16,
    current_agc_gain: i16,
    last_agc_update: Instant,
    if_gain_mode: IfGainMode,
    mode: Mode,
    user_power: RfPowerPercent,
    budget_cp: i32,
    thermal_cp: i32,
    alc_cp: i32,
    last_contract: PdContract,
}

impl IfAmplifier {
    pub fn new(
        i2c: &'static MainBoardI2CMutex,
        dac_addr: I2cAddress,
        adc_addr: I2cAddress,
    ) -> Self {
        Self {
            dac: MCP4728::new(dac_addr.into(), i2c),
            adc_rssi: ADS1015::new(adc_addr.into(), i2c),
            if_gain_mode: IfGainMode::Manual,
            rssi_dbm: RssiDbm { dbm: 0 },
            desired_manual_gain: 0,
            current_agc_gain: MAX_GAIN / 2,
            last_agc_update: Instant::now(),
            mode: Mode::StandBy,
            user_power: RfPowerPercent::new(0),
            budget_cp: 10000,
            thermal_cp: 10000,
            alc_cp: 10000,
            last_contract: PdContract::default(),
        }
    }

    pub async fn set_manual_gain_raw(&mut self, gain: IfGain) -> Result<(), &'static str> {
        self.desired_manual_gain = ((gain.raw().max(0) as u32 * DAC_MAX as u32) / 1000) as i16;
        self.update_outputs().await
    }

    pub async fn set_if_gain_mode(&mut self, if_gain_mode: IfGainMode) -> Result<(), &'static str> {
        self.if_gain_mode = if_gain_mode;
        self.update_outputs().await
    }

    pub async fn set_mode(&mut self, mode: Mode) -> Result<(), &'static str> {
        self.mode = mode;
        if mode == Mode::WarmUp {
            self.init().await?;
        }
        self.update_outputs().await
    }

    pub async fn update_agc(&mut self, rssi: RssiDbm) -> Result<(), &'static str> {
        self.rssi_dbm = rssi;
        self.update_outputs().await
    }

    pub async fn set_power(&mut self, power: RfPowerPercent) -> Result<(), &'static str> {
        self.user_power = power;
        self.update_outputs().await
    }

    pub async fn set_power_telemetry(
        &mut self,
        telemetry: PowerTelemetry,
    ) -> Result<(), &'static str> {
        self.budget_cp = telemetry.power_budget(&self.last_contract);
        self.update_outputs().await
    }

    pub async fn set_pd_contract(&mut self, contract: PdContract) -> Result<(), &'static str> {
        self.last_contract = contract;
        self.update_outputs().await
    }

    pub async fn set_thermal_constraint(&mut self, thermal: i32) -> Result<(), &'static str> {
        self.thermal_cp = thermal;
        self.update_outputs().await
    }

    pub async fn set_alc_constraint(&mut self, alc: i32) -> Result<(), &'static str> {
        self.alc_cp = alc;
        self.update_outputs().await
    }

    pub async fn read_rssi(&mut self) -> Result<RssiData, &'static str> {
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
        Ok(RssiData {
            rssi1: RssiDbm::from_adc_raw(rssi1),
            rssi2: RssiDbm::from_adc_raw(rssi2),
        })
    }

    async fn update_outputs(&mut self) -> Result<(), &'static str> {
        match self.mode {
            Mode::StandBy | Mode::WarmUp => Ok(()),
            Mode::Rx => self.update_rx().await,
            Mode::Tx => self.update_tx().await,
        }
    }

    async fn update_rx(&mut self) -> Result<(), &'static str> {
        let gain = match self.if_gain_mode {
            IfGainMode::Manual => self.desired_manual_gain,
            IfGainMode::AgcFast => self.calculate_agc_gain(false),
            IfGainMode::AgcSlow => self.calculate_agc_gain(true),
        };
        let gain = gain.clamp(MIN_GAIN, MAX_GAIN) as u32;

        let (agc1, agc2) = distribute_gain(gain);

        let values = channel_values(agc1, agc2, DAC_MAX);
        self.dac
            .fast_write(values)
            .await
            .map_err(|_| "Failed to set IF amplifier DAC")
    }

    async fn update_tx(&mut self) -> Result<(), &'static str> {
        let effective = self.effective_power();
        let dac_value = ((effective as u32 * DAC_MAX as u32) / CENTIPERCENT_MAX) as u16;

        let (agc1, agc2) = distribute_gain(dac_value as u32);

        let values = channel_values(agc1, agc2, 0);
        self.dac
            .fast_write(values)
            .await
            .map_err(|_| "Failed to set IF amplifier DAC")
    }

    fn effective_power(&self) -> u16 {
        let limit = self.budget_cp.min(self.thermal_cp).min(self.alc_cp).max(0) as u16;
        self.user_power.centipercent.min(limit)
    }

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
        self.dac
            .ensure_address()
            .await
            .map_err(|_| "Failed to program MCP4728 address")?;

        self.adc_rssi
            .init()
            .await
            .map_err(|_| "Failed to init RSSI ADC")?;
        let values = channel_values(0, 0, 0);
        self.dac
            .fast_write(values)
            .await
            .map_err(|_| "Failed to init IF amplifier DAC")?;
        Ok(())
    }
}

fn distribute_gain(gain: u32) -> (u16, u16) {
    let total = gain * GAIN_TOTAL_RANGE / DAC_MAX as u32;
    let agc1 = total.min(DAC_MAX as u32) as u16;
    let agc2 = total.saturating_sub(DAC_MAX as u32).min(DAC_MAX as u32) as u16;
    (agc1, agc2)
}

fn channel_values(agc1: u16, agc2: u16, rx: u16) -> [u16; 4] {
    let mut values = [0u16; 4];
    values[CH_AGC2 as usize] = agc2;
    values[CH_AGC1 as usize] = agc1;
    values[CH_RX as usize] = rx;
    values
}
