use crate::app::types::{ClarifierMode, ClarifierValue, FilterType, Mode};
use crate::i2c_map::I2cAddress;
use crate::main_board::types::{MainBoardI2C, MainBoardI2CMutex};
use common::drivers::ad9834::{AD9834Config, AD9834};
use common::drivers::sc18is602::{SC18IS602SpiDevice, SC18IS602};

const REFERENCE_CLOCK_HZ: u32 = 75_000_000;
const SIGN_CHANGE_FREQUENCY: u32 = 12_000_000; // 12 MHz
const FSYNC_GPIO_PIN: u8 = 0;

pub struct Mixer {
    bridge: SC18IS602SpiDevice<MainBoardI2C>,
    synthesizer: AD9834,
    vfo_frequency: u32,
    filter: FilterType,
    mode: Mode,
    clarifier_mode: ClarifierMode,
    clarifier_value: ClarifierValue,
}

impl Mixer {
    pub fn new(i2c: &'static MainBoardI2CMutex, bridge_addr: I2cAddress) -> Self {
        let bridge = SC18IS602SpiDevice::new(
            SC18IS602::new(bridge_addr.into(), i2c),
            FSYNC_GPIO_PIN,
        );

        let synthesizer = AD9834::new(AD9834Config {
            reference_clock_hz: REFERENCE_CLOCK_HZ,
            enable_doubler: true,
        });

        Self {
            bridge,
            synthesizer,
            vfo_frequency: 0,
            filter: FilterType::Narrow,
            mode: Mode::StandBy,
            clarifier_mode: ClarifierMode::Off,
            clarifier_value: ClarifierValue::new(0),
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

    async fn update_state(&mut self) -> Result<(), &'static str> {
        match self.mode {
            Mode::Rx | Mode::Tx => {
                let dds_frequency = self.calculate_dds_frequency();

                self.synthesizer
                    .set_frequency(&mut self.bridge, dds_frequency)
                    .await
                    .map_err(|_| "Failed to set DDS frequency")?;

                Ok(())
            }
            Mode::WarmUp => self.init().await,
            Mode::StandBy => Ok(()),
        }
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

    async fn init(&mut self) -> Result<(), &'static str> {
        self.bridge
            .init()
            .await
            .map_err(|_| "Failed to initialize bridge of dds")?;
        self.bridge
            .bridge
            .set_gpio_direction(0xFF)
            .await
            .map_err(|_| "Failed to initialize bridge of dds")?;

        self.synthesizer
            .init(&mut self.bridge)
            .await
            .map_err(|_| "Failed to initialize ad9851")?;
        Ok(())
    }
}
