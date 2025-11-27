/*
 * IF Filter Selection
 *
 * Manages switching between narrow and wide IF filters.
 * Uses PCA9534 GPIO expander for relay control.
 *
 * Pin mapping (configurable via main_board::config):
 * - Pin0: Single filter relay
 * - Pin1: DoubleNarrow filter relay
 * - Pin2: DoubleWide filter relay
 * - Pin3: +RX power enable
 */

use crate::{
    app::types::{FilterType, Mode},
    main_board::{
        config::{FilterSelectPins, FILTER_SELECT_I2C_ADDR},
        types::{MainBoardI2C, MainBoardI2CMutex},
    },
};
use common::drivers::pca9534::PCA9534;

pub struct FilterSelect {
    gpio: PCA9534<MainBoardI2C>,
    pins: FilterSelectPins,
    filter: FilterType,
    mode: Mode,
}

impl FilterSelect {
    pub fn new(i2c: &'static MainBoardI2CMutex, initial_filter: FilterType) -> Self {
        Self {
            gpio: PCA9534::new(FILTER_SELECT_I2C_ADDR, i2c),
            pins: FilterSelectPins::default(),
            filter: initial_filter,
            mode: Mode::StandBy,
        }
    }

    pub async fn set_filter(&mut self, filter: FilterType) -> Result<(), &'static str> {
        self.filter = filter;
        self.apply_filter_setting().await
    }

    pub async fn set_mode(&mut self, mode: Mode) -> Result<(), &'static str> {
        self.mode = mode;
        self.apply_filter_setting().await
    }

    // TODO check this shit
    async fn apply_filter_setting(&mut self) -> Result<(), &'static str> {
        // Determine which pins should be active
        let (filter_pin, rx_enable) = match self.mode {
            Mode::Rx => {
                // In RX mode: activate the selected filter and +RX
                let pin = self.pins.get_filter_pin(self.filter);
                (Some(pin), true)
            }
            Mode::Tx | Mode::StandBy | Mode::WarmUp => {
                // In other modes: disable all filters and +RX
                (None, false)
            }
        };

        // Build the output byte
        let mut output: u8 = 0x00;

        // Set filter relay
        if let Some(pin) = filter_pin {
            output |= pin.mask();
        }

        // Set +RX enable
        if rx_enable {
            output |= self.pins.rx_enable.mask();
        }

        // Write to GPIO expander
        self.gpio
            .write_port(output)
            .await
            .map_err(|_| "Failed to set filter relays")?;

        Ok(())
    }
}
