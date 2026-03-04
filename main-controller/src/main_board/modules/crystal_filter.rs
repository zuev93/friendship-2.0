use crate::app::types::{Mode, NbLevel};
use crate::i2c_map::I2cAddress;
use crate::main_board::types::{MainBoardI2C, MainBoardI2CMutex, RssiDbm};
use common::drivers::mcp4725::MCP4725;
use common::drivers::pca9534::{Pin, PCA9534};

const DAC_12BIT_MAX: u32 = 4095;

const IO_NB_ACTIVITY_PIN: Pin = Pin::Pin0;

const VDD_MV: u32 = 3300;
const NB_DELTA_MIN_MV: u32 = 50;
const NB_DELTA_MAX_MV: u32 = 1500;
const NB_LEVEL_MAX: u32 = 1000;
const ADC_MAX_RAW: i32 = 1656;
const ADC_FS_MV: i32 = 4096;

pub struct CrystalFilter {
    io: PCA9534<MainBoardI2C>,
    nb_dac: MCP4725<MainBoardI2C>,
    mode: Mode,
    nb_level: NbLevel,
    last_rssi: RssiDbm,
}

impl CrystalFilter {
    pub fn new(
        i2c: &'static MainBoardI2CMutex,
        pca9534_addr: I2cAddress,
        nb_mcp4725_addr: I2cAddress,
    ) -> Self {
        let io = PCA9534::new(pca9534_addr.into(), i2c);
        let nb_dac = MCP4725::new(nb_mcp4725_addr.into(), i2c);
        Self {
            io,
            nb_dac,
            mode: Mode::StandBy,
            nb_level: NbLevel::new(0),
            last_rssi: RssiDbm { dbm: -120 },
        }
    }

    pub async fn set_mode(&mut self, mode: Mode) -> Result<(), &'static str> {
        let prev = self.mode;
        self.mode = mode;
        if mode == Mode::WarmUp && prev != Mode::WarmUp {
            self.init().await?;
        }
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

    async fn update_nb_threshold(&mut self) -> Result<(), &'static str> {
        if self.mode == Mode::Tx || self.mode == Mode::StandBy {
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
            .set_direction(0x01)
            .await
            .map_err(|_| "Failed to init crystal filter IO")?;
        Ok(())
    }
}

fn rssi_to_mv(rssi: RssiDbm) -> u32 {
    let raw_estimate = ((rssi.dbm as i32 + 120) * ADC_MAX_RAW / 100).max(0);
    let mv = (raw_estimate * ADC_FS_MV / 2048).max(0);
    mv as u32
}

fn nb_delta_mv(nb_level: NbLevel) -> u32 {
    let level = nb_level.raw().max(0) as u32;
    NB_DELTA_MIN_MV + (level * (NB_DELTA_MAX_MV - NB_DELTA_MIN_MV) / NB_LEVEL_MAX)
}
