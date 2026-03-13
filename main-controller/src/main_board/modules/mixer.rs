use crate::app::types::{ClarifierMode, ClarifierValue, FilterType, Mode, TransmitMode};
use crate::main_board::types::MainBoardI2C;
use common::drivers::si5351::{ClkOutput, PllSource, Si5351};

const SIGN_CHANGE_FREQUENCY: u32 = 12_000_000;
const AUDIO_LOW_HZ: u32 = 300;
const CW_OFFSET_HZ: u32 = 700;

pub struct Mixer {
    si5351: Si5351<MainBoardI2C>,
    vfo_frequency: u32,
    filter: FilterType,
    mode: Mode,
    transmit_mode: TransmitMode,
    clarifier_mode: ClarifierMode,
    clarifier_value: ClarifierValue,
}

impl Mixer {
    pub fn new(si5351: Si5351<MainBoardI2C>) -> Self {
        Self {
            si5351,
            vfo_frequency: 0,
            filter: FilterType::Narrow,
            mode: Mode::StandBy,
            transmit_mode: TransmitMode::Lsb,
            clarifier_mode: ClarifierMode::Off,
            clarifier_value: ClarifierValue::new(0),
        }
    }

    pub async fn set_frequency(&mut self, frequency_hz: u32) -> Result<(), &'static str> {
        self.vfo_frequency = frequency_hz;
        self.update_vfo().await
    }

    pub async fn set_filter(&mut self, filter: FilterType) -> Result<(), &'static str> {
        self.filter = filter;
        self.update_vfo().await?;
        self.update_bfo().await
    }

    pub async fn set_transmit_mode(
        &mut self,
        transmit_mode: TransmitMode,
    ) -> Result<(), &'static str> {
        self.transmit_mode = transmit_mode;
        self.update_bfo().await
    }

    pub async fn set_clarifier_mode(
        &mut self,
        clarifier_mode: ClarifierMode,
    ) -> Result<(), &'static str> {
        self.clarifier_mode = clarifier_mode;
        self.update_vfo().await
    }

    pub async fn set_clarifier_value(
        &mut self,
        clarifier_value: ClarifierValue,
    ) -> Result<(), &'static str> {
        self.clarifier_value = clarifier_value;
        self.update_vfo().await
    }

    pub async fn set_mode(&mut self, mode: Mode) -> Result<(), &'static str> {
        self.mode = mode;
        match mode {
            Mode::WarmUp => {
                self.si5351.init().await.map_err(|_| "Si5351 init failed")?;
                Ok(())
            }
            Mode::Rx | Mode::Tx => {
                self.update_vfo().await?;
                self.update_bfo().await
            }
            Mode::StandBy => Ok(()),
        }
    }

    async fn update_vfo(&mut self) -> Result<(), &'static str> {
        match self.mode {
            Mode::Rx | Mode::Tx => {
                let lo_frequency = self.calculate_lo_frequency();
                self.si5351
                    .set_frequency(PllSource::PllA, ClkOutput::Clk0, lo_frequency)
                    .await
                    .map_err(|_| "Failed to set VFO frequency")
            }
            Mode::WarmUp | Mode::StandBy => Ok(()),
        }
    }

    async fn update_bfo(&mut self) -> Result<(), &'static str> {
        match self.mode {
            Mode::Rx | Mode::Tx => {
                let bfo_frequency = self.calculate_bfo_frequency();
                self.si5351
                    .set_frequency_iq_pair(PllSource::PllB, bfo_frequency)
                    .await
                    .map_err(|_| "Failed to set BFO I/Q frequency")
            }
            Mode::WarmUp | Mode::StandBy => Ok(()),
        }
    }

    fn calculate_lo_frequency(&self) -> u32 {
        let base_freq = if self.vfo_frequency > SIGN_CHANGE_FREQUENCY {
            self.vfo_frequency - self.filter.center_frequency_hz()
        } else {
            self.vfo_frequency + self.filter.center_frequency_hz()
        };
        match self.mode {
            Mode::StandBy | Mode::WarmUp => 0,
            Mode::Rx => {
                if self.clarifier_mode == ClarifierMode::Rit {
                    base_freq.saturating_add_signed(self.clarifier_value.raw() as i32)
                } else {
                    base_freq
                }
            }
            Mode::Tx => {
                if self.clarifier_mode == ClarifierMode::XIT {
                    base_freq.saturating_add_signed(self.clarifier_value.raw() as i32)
                } else {
                    base_freq
                }
            }
        }
    }

    fn calculate_bfo_frequency(&self) -> u32 {
        let filter_center = self.filter.center_frequency_hz();
        let filter_bw = self.filter.bandwidth_hz();
        let bfo_offset = (filter_bw / 2) + AUDIO_LOW_HZ;

        match self.transmit_mode {
            TransmitMode::Usb => filter_center - bfo_offset,
            TransmitMode::Lsb => filter_center + bfo_offset,
            TransmitMode::Cw => filter_center - bfo_offset + CW_OFFSET_HZ,
            TransmitMode::Am => filter_center,
        }
    }
}
