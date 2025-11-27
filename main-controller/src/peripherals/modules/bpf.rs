/*
 * BPF (Band Pass Filter) Module
 *
 * Controls input band-pass filters for different amateur radio bands
 * Uses TCA9555 16-bit I2C GPIO expander for relay control
 * Port0: 8 band-pass filters (160m-15m)
 * Port1: 1 filter (12m) + control pins (ATT, RF Amp, TX bypass)
 */

use crate::app::types::{Frequency, Mode, RfGainMode};
use crate::peripherals::config::BpfConfig;
use crate::peripherals::types::PeripherialI2cMutex;
use common::drivers::tca9555::{Port, TCA9555};

const BPF_GPIO_ADDR: u8 = 0x21; // TODO: Move to i2c_map

pub struct Bpf {
    i2c: PeripherialI2cMutex,
    bpf_config: BpfConfig,
    gpio: TCA9555,
    mode: Mode,
    rf_gain_mode: RfGainMode,
    frequency: Frequency,
}

impl Bpf {
    pub fn new(i2c: PeripherialI2cMutex, bpf_config: BpfConfig) -> Self {
        Self {
            i2c,
            bpf_config,
            gpio: TCA9555::new(BPF_GPIO_ADDR),
            mode: Mode::StandBy,
            rf_gain_mode: RfGainMode::Normal,
            frequency: 0,
        }
    }

    pub async fn set_mode(&mut self, mode: Mode) -> Result<(), &'static str> {
        self.mode = mode;
        self.update_state().await
    }

    pub async fn set_frequency(&mut self, frequency: Frequency) -> Result<(), &'static str> {
        self.frequency = frequency;
        self.update_state().await
    }

    pub async fn set_rf_gain_mode(&mut self, rf_gain_mode: RfGainMode) -> Result<(), &'static str> {
        self.rf_gain_mode = rf_gain_mode;
        self.update_state().await
    }

    pub async fn update_state(&mut self) -> Result<(), &'static str> {
        let mut i2c_guard = self.i2c.lock().await;

        match self.mode {
            Mode::StandBy => {
                // Turn off all pins in standby mode
                self.gpio
                    .write_port(&mut *i2c_guard, Port::Port0, 0x00)
                    .await
                    .map_err(|_| "Failed to turn off BPF Port0")?;
                self.gpio
                    .write_port(&mut *i2c_guard, Port::Port1, 0x00)
                    .await
                    .map_err(|_| "Failed to turn off BPF Port1")?;
                return Ok(());
            }
            Mode::WarmUp => {
                self.gpio
                    .init(&mut *i2c_guard)
                    .await
                    .map_err(|_| "Failed to initialize BPF GPIO")?;

                // Set all pins as outputs on both ports
                self.gpio
                    .set_port_direction(&mut *i2c_guard, Port::Port0, 0x00)
                    .await
                    .map_err(|_| "Failed to set BPF Port0 direction")?;
                self.gpio
                    .set_port_direction(&mut *i2c_guard, Port::Port1, 0x00)
                    .await
                    .map_err(|_| "Failed to set BPF Port1 direction")?;

                // Clear all outputs
                self.gpio
                    .write_port(&mut *i2c_guard, Port::Port0, 0x00)
                    .await
                    .map_err(|_| "Failed to clear BPF Port0")?;
                self.gpio
                    .write_port(&mut *i2c_guard, Port::Port1, 0x00)
                    .await
                    .map_err(|_| "Failed to clear BPF Port1")?;

                return Ok(());
            }
            _ => {}
        }

        // Build port values using TCA9555 helper function
        let mut port0_value: u8 = 0;
        let mut port1_value: u8 = 0;

        // Set the appropriate band pass filter pin
        let (filter_port, filter_pin) = self.bpf_config.find_filter(self.frequency);
        (port0_value, port1_value) =
            TCA9555::set_pin_value(port0_value, port1_value, filter_port, filter_pin, true);

        // Set TX bypass pin (Port1)
        if self.mode == Mode::Tx {
            (port0_value, port1_value) = TCA9555::set_pin_value(
                port0_value,
                port1_value,
                self.bpf_config.control.tx_port,
                self.bpf_config.control.tx_pin,
                true,
            );
        }

        // Set ATT/RF Amp pins based on RF gain mode (both on Port1)
        match self.rf_gain_mode {
            RfGainMode::Attenuator => {
                (port0_value, port1_value) = TCA9555::set_pin_value(
                    port0_value,
                    port1_value,
                    self.bpf_config.control.att_port,
                    self.bpf_config.control.att_pin,
                    true,
                );
            }
            RfGainMode::Normal | RfGainMode::RfSingle => {
                // Both ATT and RF amp are off (already 0)
            }
            RfGainMode::RfDouble => {
                (port0_value, port1_value) = TCA9555::set_pin_value(
                    port0_value,
                    port1_value,
                    self.bpf_config.control.rf_amp_port,
                    self.bpf_config.control.rf_amp_pin,
                    true,
                );
            }
        }

        // Write both ports in two operations
        self.gpio
            .write_port(&mut *i2c_guard, Port::Port0, port0_value)
            .await
            .map_err(|_| "Failed to write BPF Port0")?;
        self.gpio
            .write_port(&mut *i2c_guard, Port::Port1, port1_value)
            .await
            .map_err(|_| "Failed to write BPF Port1")?;

        Ok(())
    }
}
