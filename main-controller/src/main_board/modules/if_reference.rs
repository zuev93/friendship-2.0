/*
 * IF Reference Generator
 *
 * Generates Beat Frequency Oscillator signal for SSB/CW/AM detection.
 * Uses AD9834 DDS with built-in frequency doubler for precise frequency synthesis.
 *
 * IF Filter: 10 MHz center, 2.4 kHz bandwidth
 * BFO positions relative to filter center (Fc ± 1.5 kHz):
 * - USB: BFO = Fc - (BW/2 + f_low) = 9.998500 MHz (below center)
 * - LSB: BFO = Fc + (BW/2 + f_low) = 10.001500 MHz (above center)
 * - CW:  BFO = USB + 700 Hz tone   = 9.999200 MHz
 * - AM:  BFO = Fc                  = 10.000000 MHz (carrier detection)
 */

use crate::app::types::{FilterType, Mode, TransmitMode};
use common::drivers::ad9834::{AD9834Config, Waveform, AD9834};
use common::drivers::sc18is602::{SC18IS602SpiDevice, SC18IS602};
use crate::i2c_map;

use crate::main_board::types::MainBoardI2CMutex;

// TODO move to settings
const AUDIO_LOW_HZ: u32 = 300;
const CW_OFFSET_HZ: u32 = 700;

const REFERENCE_CLOCK_HZ: u32 = 20_000_000; // 20 MHz TCXO (same as AD9851)
const ENABLE_DOUBLER: bool = true; // Enable doubler for full 20 MHz range

pub struct IfReference {
    dds: AD9834,
    sc18is602: SC18IS602,
    i2c: &'static MainBoardI2CMutex,
    mode: Mode,
    transmit_mode: TransmitMode,
    current_filter: FilterType,
}

impl IfReference {
    pub fn new(
        i2c: &'static MainBoardI2CMutex,
        transmit_mode: TransmitMode,
        filter: FilterType,
    ) -> Self {
        let dds_config = AD9834Config {
            reference_clock_hz: REFERENCE_CLOCK_HZ,
            enable_doubler: ENABLE_DOUBLER,
        };
        let dds = AD9834::new(dds_config);
        let sc18is602 = SC18IS602::new(i2c_map::SC18IS602_IF_REF_ADDR);

        Self {
            i2c,
            dds,
            sc18is602,
            mode: Mode::StandBy,
            transmit_mode: transmit_mode,
            current_filter: filter,
        }
    }

    pub async fn set_mode(&mut self, mode: Mode) -> Result<(), &'static str> {
        self.mode = mode;
        self.update_state().await
    }

    pub async fn set_transmit_mode(
        &mut self,
        transmit_mode: TransmitMode,
    ) -> Result<(), &'static str> {
        self.transmit_mode = transmit_mode;
        self.update_state().await
    }

    pub async fn set_filter(&mut self, filter: FilterType) -> Result<(), &'static str> {
        self.current_filter = filter;
        self.update_state().await
    }

    async fn update_state(&mut self) -> Result<(), &'static str> {
        let frequency_hz = self.calculate_bfo_frequency();
        let mut i2c_guard = self.i2c.lock().await;
        let mut bridge = SC18IS602SpiDevice::new(&mut self.sc18is602, &mut *i2c_guard, 0);

        match self.mode {
            Mode::Rx | Mode::Tx => self
                .dds
                .set_frequency(&mut bridge, frequency_hz)
                .await
                .map_err(|_| "Failed to set frequency of if reference"),
            Mode::WarmUp => {
                self.dds
                    .init(&mut bridge)
                    .await
                    .map_err(|_| "Failed to initialize IF reference generator")?;

                self.dds
                    .set_waveform(&mut bridge, Waveform::Sine)
                    .await
                    .map_err(|_| "Failed to set waveform of if reference")
            }
            Mode::StandBy => {
                // TODO turn of
                Ok(())
            }
        }
    }

    fn calculate_bfo_frequency(&mut self) -> u32 {
        let filter_center = self.current_filter.center_frequency_hz();
        let filter_bw = self.current_filter.bandwidth_hz();

        let bfo_offset = (filter_bw / 2) + AUDIO_LOW_HZ;

        match self.transmit_mode {
            TransmitMode::Usb => filter_center - bfo_offset,
            TransmitMode::Lsb => filter_center + bfo_offset,
            TransmitMode::Cw => filter_center - bfo_offset + CW_OFFSET_HZ,
            TransmitMode::Am => filter_center,
        }
    }
}
