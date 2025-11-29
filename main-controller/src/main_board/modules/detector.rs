use crate::app::types::{FilterType, Mode, TransmitMode};
use crate::i2c_map;
use common::drivers::ad9834::{AD9834Config, Waveform, AD9834};
use common::drivers::sc18is602::{GpioPin, SC18IS602SpiDevice, SC18IS602};

use crate::main_board::types::{MainBoardI2C, MainBoardI2CMutex};

// TODO move to settings
const AUDIO_LOW_HZ: u32 = 300;
const CW_OFFSET_HZ: u32 = 700;

const REFERENCE_CLOCK_HZ: u32 = 75_000_000;
const ENABLE_DOUBLER: bool = true;

const IO_TX_PIN: GpioPin = GpioPin::P2;
const IO_RX_PIN: GpioPin = GpioPin::P3;

pub struct Detector {
    bridge: SC18IS602SpiDevice<MainBoardI2C>,
    dds: AD9834,
    mode: Mode,
    transmit_mode: TransmitMode,
    current_filter: FilterType,
}

impl Detector {
    pub fn new(i2c: &'static MainBoardI2CMutex) -> Self {
        let dds_config = AD9834Config {
            reference_clock_hz: REFERENCE_CLOCK_HZ,
            enable_doubler: ENABLE_DOUBLER,
        };
        let dds = AD9834::new(dds_config);
        let sc18is602 = SC18IS602::new(i2c_map::DETECTOR_SC18IS602_ADDR, i2c);
        let bridge: SC18IS602SpiDevice<
            embassy_stm32::i2c::I2c<
                'static,
                embassy_stm32::mode::Async,
                embassy_stm32::i2c::Master,
            >,
        > = SC18IS602SpiDevice::new(sc18is602, 0);

        Self {
            bridge,
            dds,
            mode: Mode::StandBy,
            transmit_mode: TransmitMode::Lsb,
            current_filter: FilterType::None,
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
        if self.mode == Mode::StandBy {
            return Ok(());
        }
        if self.mode == Mode::WarmUp {
            return self.init().await;
        }

        let frequency_hz = self.calculate_bfo_frequency();
        self.bridge
            .bridge
            .write_gpio_pin(IO_RX_PIN, self.mode == Mode::Rx)
            .await
            .map_err(|_| "Failed to write IO detector")?;
        self.bridge
            .bridge
            .write_gpio_pin(IO_TX_PIN, self.mode == Mode::Tx)
            .await
            .map_err(|_| "Failed to write IO detector")?;
        self.dds
            .set_frequency(&mut self.bridge, frequency_hz)
            .await
            .map_err(|_| "Failed to set frequency of if reference")?;
        Ok(())
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

    async fn init(&mut self) -> Result<(), &'static str> {
        self.bridge
            .init()
            .await
            .map_err(|_| "Detector IO bridge init failed")?;
        self.bridge
            .bridge
            .set_gpio_direction(0xFF)
            .await
            .map_err(|_| "Detector IO bridge init failed")?;

        self.dds
            .init(&mut self.bridge)
            .await
            .map_err(|_| "Failed to initialize IF reference generator")?;

        self.dds
            .set_waveform(&mut self.bridge, Waveform::Sine)
            .await
            .map_err(|_| "Failed to set waveform of if reference")
    }
}
