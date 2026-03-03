use crate::app::types::{FilterType, Mode, NbLevel, RfGainMode, RfPowerPercent};
use crate::control_board::events::{PdContract, PowerTelemetry};
use crate::i2c_map::I2cAddress;
use crate::main_board::types::{MainBoardI2C, MainBoardI2CMutex, RssiDbm};
use common::drivers::mcp4725::MCP4725;
use common::drivers::pca9534::{Pin, PCA9534};

const DAC_12BIT_MAX: u32 = 4095;
const CENTIPERCENT_MAX: u32 = 10000;

const IO_RX_PIN: Pin = Pin::Pin0;
const IO_TX_PIN: Pin = Pin::Pin1;
const IO_F1_PIN: Pin = Pin::Pin2;
const IO_F2_PIN: Pin = Pin::Pin3;
const IO_AMP_EN_PIN: Pin = Pin::Pin4;
const IO_AMP_OFF_PIN: Pin = Pin::Pin5;
const IO_NB_ACTIVITY_PIN: Pin = Pin::Pin6;

const VDD_MV: u32 = 3300;
const NB_DELTA_MIN_MV: u32 = 50;
const NB_DELTA_MAX_MV: u32 = 1500;
const NB_LEVEL_MAX: u32 = 1000;
const ADC_MAX_RAW: i32 = 26500;
const ADC_FS_MV: i32 = 4096;

pub struct CrystallFilter {
    dac: MCP4725<MainBoardI2C>,
    io: PCA9534<MainBoardI2C>,
    nb_dac: MCP4725<MainBoardI2C>,
    user_power: RfPowerPercent,
    budget_cp: i32,
    thermal_cp: i32,
    alc_cp: i32,
    last_contract: PdContract,
    mode: Mode,
    filter_type: FilterType,
    rf_gain_mode: RfGainMode,
    nb_enabled: bool,
    nb_level: NbLevel,
    last_rssi: RssiDbm,
}

impl CrystallFilter {
    pub fn new(
        i2c: &'static MainBoardI2CMutex,
        mcp4725_addr: I2cAddress,
        pca9534_addr: I2cAddress,
        nb_mcp4725_addr: I2cAddress,
    ) -> Self {
        let dac = MCP4725::new(mcp4725_addr.into(), i2c);
        let io = PCA9534::new(pca9534_addr.into(), i2c);
        let nb_dac = MCP4725::new(nb_mcp4725_addr.into(), i2c);
        Self {
            io,
            dac,
            nb_dac,
            user_power: RfPowerPercent::new(0),
            budget_cp: 10000,
            thermal_cp: 10000,
            alc_cp: 10000,
            last_contract: PdContract::default(),
            mode: Mode::StandBy,
            filter_type: FilterType::Narrow,
            rf_gain_mode: RfGainMode::Attenuator,
            nb_enabled: false,
            nb_level: NbLevel::new(0),
            last_rssi: RssiDbm { dbm: -120 },
        }
    }

    pub async fn set_power(&mut self, power: RfPowerPercent) -> Result<(), &'static str> {
        self.user_power = power;
        self.update_state().await
    }

    pub async fn set_power_telemetry(
        &mut self,
        telemetry: PowerTelemetry,
    ) -> Result<(), &'static str> {
        self.budget_cp = telemetry.power_budget(&self.last_contract);
        self.update_state().await
    }

    pub async fn set_pd_contract(&mut self, contract: PdContract) -> Result<(), &'static str> {
        self.last_contract = contract;
        self.update_state().await
    }

    pub async fn set_thermal_constraint(&mut self, thermal: i32) -> Result<(), &'static str> {
        self.thermal_cp = thermal;
        self.update_state().await
    }

    pub async fn set_alc_constraint(&mut self, alc: i32) -> Result<(), &'static str> {
        self.alc_cp = alc;
        self.update_state().await
    }

    fn power_constraint(&self) -> i32 {
        self.budget_cp.min(self.thermal_cp).min(self.alc_cp)
    }

    fn effective_power(&self) -> u16 {
        let limit = self.power_constraint().max(0) as u16;
        self.user_power.centipercent.min(limit)
    }

    pub async fn set_mode(&mut self, mode: Mode) -> Result<(), &'static str> {
        self.mode = mode;
        self.update_state().await?;
        self.update_nb_threshold().await
    }

    pub async fn set_filter_type(&mut self, filter_type: FilterType) -> Result<(), &'static str> {
        self.filter_type = filter_type;
        self.update_state().await
    }

    pub async fn set_rf_gain_mode(&mut self, rf_gain_mode: RfGainMode) -> Result<(), &'static str> {
        self.rf_gain_mode = rf_gain_mode;
        self.update_state().await
    }

    pub async fn set_nb_enabled(&mut self, enabled: bool) -> Result<(), &'static str> {
        self.nb_enabled = enabled;
        self.update_nb_threshold().await
    }

    pub async fn set_nb_level(&mut self, nb_level: NbLevel) -> Result<(), &'static str> {
        self.nb_level = nb_level;
        self.update_nb_threshold().await
    }

    pub async fn set_rssi(&mut self, rssi: RssiDbm) -> Result<(), &'static str> {
        self.last_rssi = rssi;
        self.update_nb_threshold().await
    }

    pub async fn read_nb_activity(&self) -> Result<bool, &'static str> {
        let pin_high = self
            .io
            .read_pin(IO_NB_ACTIVITY_PIN)
            .await
            .map_err(|_| "Failed to read NB activity")?;
        Ok(!pin_high)
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
        if self.mode == Mode::Tx {
            port |= IO_TX_PIN.mask();
        }
        match self.filter_type {
            FilterType::Wide => {
                port |= IO_F2_PIN.mask();
            }
            FilterType::Medium | FilterType::Narrow | FilterType::CwNarrow => {
                port |= IO_F1_PIN.mask();
            }
        }
        let amp_enabled =
            self.rf_gain_mode == RfGainMode::RfSingle || self.rf_gain_mode == RfGainMode::RfDouble;
        if amp_enabled {
            port |= IO_AMP_EN_PIN.mask();
        } else {
            port |= IO_AMP_OFF_PIN.mask();
        }

        self.io
            .write_port(port)
            .await
            .map_err(|_| "Failed to write filter IO")?;

        if self.mode == Mode::Tx {
            let dac_value =
                ((self.effective_power() as u32 * DAC_12BIT_MAX) / CENTIPERCENT_MAX) as u16;
            self.dac
                .set_raw(dac_value)
                .await
                .map_err(|_| "Failed to set TX power")?;
        } else {
            self.dac
                .write_eeprom_power_down()
                .await
                .map_err(|_| "Failed to write EEPROM power down")?;
        }
        Ok(())
    }

    async fn update_nb_threshold(&mut self) -> Result<(), &'static str> {
        if !self.nb_enabled || self.mode == Mode::Tx || self.mode == Mode::StandBy {
            self.nb_dac
                .set_raw(DAC_12BIT_MAX as u16)
                .await
                .map_err(|_| "Failed to set NB threshold DAC")?;
            return Ok(());
        }

        let rssi_mv = rssi_to_mv(self.last_rssi);
        let delta_mv = nb_delta_mv(self.nb_level);
        let threshold_mv = rssi_mv + delta_mv;
        let dac_code = (threshold_mv * DAC_12BIT_MAX / VDD_MV).min(DAC_12BIT_MAX);

        self.nb_dac
            .set_raw(dac_code as u16)
            .await
            .map_err(|_| "Failed to set NB threshold DAC")?;

        Ok(())
    }

    async fn init(&mut self) -> Result<(), &'static str> {
        self.io
            .init()
            .await
            .map_err(|_| "Failed to init crystal filter IO")?;
        self.io
            .set_direction(0x40)
            .await
            .map_err(|_| "Failed to init crystal filter IO")?;
        Ok(())
    }
}

fn rssi_to_mv(rssi: RssiDbm) -> u32 {
    let raw_estimate = ((rssi.dbm as i32 + 120) * ADC_MAX_RAW / 100).max(0);
    let mv = (raw_estimate * ADC_FS_MV / 32768).max(0);
    mv as u32
}

fn nb_delta_mv(nb_level: NbLevel) -> u32 {
    let level = nb_level.raw().max(0) as u32;
    NB_DELTA_MIN_MV + (level * (NB_DELTA_MAX_MV - NB_DELTA_MIN_MV) / NB_LEVEL_MAX)
}
