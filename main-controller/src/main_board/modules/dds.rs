/*
 * DDS (Direct Digital Synthesizer) Module
 *
 * High-level abstraction for frequency generation.
 * Manages AD9851 and SC18IS602 bridge internally.
 */

use crate::app::types::{ClarifierMode, ClarifierValue, FilterType, Mode};
use crate::i2c_map;
use crate::main_board::types::MainBoardI2CMutex;
use common::drivers::ad9851::{AD9851Config, AD9851};
use common::drivers::sc18is602::{SC18IS602SpiDevice, SC18IS602};
use embassy_stm32::gpio::{Level, Output, Pin, Speed};
use embassy_stm32::Peri;

const REFERENCE_CLOCK_HZ: u32 = 20_000_000; // 20 MHz TCXO
const REFCLK_MULTIPLIER: u8 = 6; // 6x = 120 MHz system clock
const SIGN_CHANGE_FREQUENCY: u32 = 12_000_000; // 12 MHz

pub struct DDS {
    sc18is602: SC18IS602,
    ad9851: AD9851<Output<'static>, Output<'static>>,
    i2c: &'static MainBoardI2CMutex,
    vfo_frequency: u32,
    filter: FilterType,
    mode: Mode,
    clarifier_mode: ClarifierMode,
    clarifier_value: ClarifierValue,
}

impl DDS {
    pub fn new(
        i2c: &'static MainBoardI2CMutex,
        fq_ud: Peri<'static, impl Pin>,
        reset: Peri<'static, impl Pin>,
    ) -> Self {
        let sc18is602 = SC18IS602::new(i2c_map::SC18IS602_DDS_ADDR);
        let ad9851 = AD9851::new(
            Output::new(fq_ud, Level::Low, Speed::Medium),
            Output::new(reset, Level::Low, Speed::Medium),
            AD9851Config {
                reference_clock_hz: REFERENCE_CLOCK_HZ,
                refclk_multiplier: REFCLK_MULTIPLIER,
            },
        );

        Self {
            i2c,
            sc18is602,
            ad9851,
            vfo_frequency: 0,
            filter: FilterType::Single,
            mode: Mode::StandBy,
            clarifier_mode: ClarifierMode::Off,
            clarifier_value: 0,
        }
    }

    pub async fn set_frequency(&mut self, frequency_hz: u32) -> Result<(), &'static str> {
        self.vfo_frequency = frequency_hz;
        self.update_state().await
    }

    pub async fn set_filter(&mut self, filter: FilterType) -> Result<(), &'static str> {
        self.filter = filter;
        self.update_state().await
    }

    pub async fn set_clarifier_mode(
        &mut self,
        clarifier_mode: ClarifierMode,
    ) -> Result<(), &'static str> {
        self.clarifier_mode = clarifier_mode;
        self.update_state().await
    }

    pub async fn set_clarifier_value(
        &mut self,
        clarifier_value: ClarifierValue,
    ) -> Result<(), &'static str> {
        self.clarifier_value = clarifier_value;
        self.update_state().await
    }

    pub async fn set_mode(&mut self, mode: Mode) -> Result<(), &'static str> {
        self.mode = mode;
        self.update_state().await
    }

    fn calculate_dds_frequency(&self) -> u32 {
        let base_freq = if self.vfo_frequency > SIGN_CHANGE_FREQUENCY {
            self.vfo_frequency - self.filter.center_frequency_hz()
        } else {
            self.vfo_frequency + self.filter.center_frequency_hz()
        };
        match self.mode {
            Mode::StandBy | Mode::WarmUp => 0,
            Mode::Rx => {
                if self.clarifier_mode == ClarifierMode::Rit {
                    base_freq.saturating_add_signed(self.clarifier_value as i32)
                } else {
                    base_freq
                }
            }
            Mode::Tx => {
                if self.clarifier_mode == ClarifierMode::XIT {
                    base_freq.saturating_add_signed(self.clarifier_value as i32)
                } else {
                    base_freq
                }
            }
        }
    }

    async fn update_state(&mut self) -> Result<(), &'static str> {
        match self.mode {
            Mode::Rx | Mode::Tx => {
                let dds_frequency = self.calculate_dds_frequency();

                let mut i2c_guard = self.i2c.lock().await;
                let mut bridge = SC18IS602SpiDevice::new(&mut self.sc18is602, &mut *i2c_guard, 0);

                self.ad9851
                    .set_frequency(&mut bridge, dds_frequency)
                    .await
                    .map_err(|_| "Failed to set DDS frequency")?;

                Ok(())
            }
            // TODO do standby
            Mode::StandBy | Mode::WarmUp => self.init().await,
        }
    }

    async fn init(&mut self) -> Result<(), &'static str> {
        let mut i2c_guard = self.i2c.lock().await;
        let mut bridge = SC18IS602SpiDevice::new(&mut self.sc18is602, &mut *i2c_guard, 0);

        bridge
            .init()
            .await
            .map_err(|_| "Failed to initialize bridge of dds")?;

        self.ad9851
            .init(&mut bridge)
            .await
            .map_err(|_| "Failed to initialize ad9851")?;
        Ok(())
    }
}
