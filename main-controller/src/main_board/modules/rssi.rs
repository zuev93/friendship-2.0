/*
 * RSSI (Received Signal Strength Indicator) Reader (async with DMA)
 *
 * Encapsulates ADS1115 ADC for reading RSSI signals.
 * Provides abstraction for S-meter and AGC functionality.
 */

use crate::{
    app::types::Mode,
    main_board::types::{MainBoardI2CMutex, RssiDbm},
};
use common::drivers::ads1115::{ADS1115Config, ADS1115};

#[derive(Debug, Clone, Copy)]
pub struct RssiData {
    pub rssi1: RssiDbm, // AIN0 - Primary RSSI
    pub rssi2: RssiDbm, // AIN1 - Secondary RSSI
}

const ADC_ADDRESS: u8 = crate::i2c_map::ADS1115_RSSI_ADDR;

fn get_adc_config() -> ADS1115Config {
    ADS1115Config::default()
}

pub struct RssiReader {
    adc: ADS1115,
    i2c: &'static MainBoardI2CMutex,
    mode: Mode,
}

impl RssiReader {
    pub fn new(i2c: &'static MainBoardI2CMutex) -> Self {
        Self {
            adc: ADS1115::new(ADC_ADDRESS, get_adc_config()),
            i2c,
            mode: Mode::StandBy,
        }
    }

    pub async fn read(&mut self) -> Result<RssiData, &'static str> {
        let mut i2c_guard = self.i2c.lock().await;
        async {
            let rssi1 = self
                .adc
                .read_ain0(&mut *i2c_guard)
                .await
                .map_err(|_| "Failed to read RSSI1")?;
            let rssi2 = self
                .adc
                .read_ain1(&mut *i2c_guard)
                .await
                .map_err(|_| "Failed to read RSSI2")?;
            Ok::<RssiData, &'static str>(RssiData {
                rssi1: RssiDbm::from_adc_raw(rssi1),
                rssi2: RssiDbm::from_adc_raw(rssi2),
            })
        }
        .await
    }

    pub async fn set_mode(&mut self, mode: Mode) -> Result<(), &'static str> {
        self.mode = mode;
        self.update_state().await
    }

    async fn update_state(&mut self) -> Result<(), &'static str> {
        match self.mode {
            Mode::Rx => Ok(()),
            Mode::Tx => Ok(()),
            // TODO implement me
            Mode::StandBy => Ok(()),
            Mode::WarmUp => {
                let mut i2c_guard = self.i2c.lock().await;
                self.adc
                    .init(&mut *i2c_guard)
                    .await
                    .map_err(|_| "Failed to initialize ADS1115 for RSSI reader")
            }
        }
    }
}
