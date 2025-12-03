/*
 * LPF (Low Pass Filter) Module
 * Controls low-pass filters (TCA9555) and reads coupler power/VSWR (ADS1115) on the same board.
 * TCA9555: Port0 = LPF1..LPF8 (Pin0..Pin7); Port1 Pin0 = LPF9; Pin1 = TX bypass
 * ADS1115: AIN0 = forward voltage, AIN1 = reflected voltage
 */

use crate::app::types::{CouplerMetrics, Frequency, Mode};
use crate::i2c_map::I2cAddress;
use crate::peripherals::types::{PeripherialI2c, PeripherialI2cMutex};
use common::drivers::ads1115::{ADS1115Config, ADS1115};
use common::drivers::tca9555::{Pin as TcaPin, Port as TcaPort, TCA9555};

const COUPLER_V_PER_COUNT: f32 = 4.096 / 32768.0; // ADS1115 gain 4.096V, single-ended counts
const COUPLER_W_PER_V_SQUARED: f32 = 1.0; // TODO: calibrate coupler factor

#[derive(Clone, Copy)]
struct LpfFilter {
    freq_min: Frequency,
    freq_max: Frequency,
    port: TcaPort,
    pin: TcaPin,
}

#[derive(Clone, Copy)]
struct LpfConfig {
    filters: [LpfFilter; 9],
    tx_pin: LpfFilter,
}

impl LpfConfig {
    fn default() -> Self {
        Self {
            filters: [
                LpfFilter {
                    freq_min: 1_800_000,
                    freq_max: 2_000_000,
                    port: TcaPort::Port0,
                    pin: TcaPin::Pin0,
                },
                LpfFilter {
                    freq_min: 3_500_000,
                    freq_max: 4_000_000,
                    port: TcaPort::Port0,
                    pin: TcaPin::Pin1,
                },
                LpfFilter {
                    freq_min: 5_000_000,
                    freq_max: 7_500_000,
                    port: TcaPort::Port0,
                    pin: TcaPin::Pin2,
                },
                LpfFilter {
                    freq_min: 10_000_000,
                    freq_max: 10_150_000,
                    port: TcaPort::Port0,
                    pin: TcaPin::Pin3,
                },
                LpfFilter {
                    freq_min: 14_000_000,
                    freq_max: 14_350_000,
                    port: TcaPort::Port0,
                    pin: TcaPin::Pin4,
                },
                LpfFilter {
                    freq_min: 18_000_000,
                    freq_max: 18_168_000,
                    port: TcaPort::Port0,
                    pin: TcaPin::Pin5,
                },
                LpfFilter {
                    freq_min: 21_000_000,
                    freq_max: 21_450_000,
                    port: TcaPort::Port0,
                    pin: TcaPin::Pin6,
                },
                LpfFilter {
                    freq_min: 24_000_000,
                    freq_max: 30_000_000,
                    port: TcaPort::Port0,
                    pin: TcaPin::Pin7,
                },
                LpfFilter {
                    freq_min: 50_000_000,
                    freq_max: 54_000_000,
                    port: TcaPort::Port1,
                    pin: TcaPin::Pin0,
                },
            ],
            tx_pin: LpfFilter {
                freq_min: 0,
                freq_max: 0,
                port: TcaPort::Port1,
                pin: TcaPin::Pin1,
            },
        }
    }

    fn find_filter(&self, frequency: Frequency) -> (TcaPort, TcaPin) {
        self.filters
            .iter()
            .find(|f| frequency >= f.freq_min && frequency <= f.freq_max)
            .unwrap_or_else(|| self.filters.last().expect("filters non-empty"))
            .as_port_pin()
    }
}

impl LpfFilter {
    fn as_port_pin(&self) -> (TcaPort, TcaPin) {
        (self.port, self.pin)
    }
}

pub struct Lpf {
    config: LpfConfig,
    gpio: TCA9555<PeripherialI2c>,
    adc: ADS1115<PeripherialI2c>,
    coupler_initialized: bool,
    mode: Mode,
    frequency: Frequency,
}

impl Lpf {
    pub fn new(
        i2c: PeripherialI2cMutex,
        gpio_addr: I2cAddress,
        ads1115_addr: I2cAddress,
    ) -> Self {
        Self {
            config: LpfConfig::default(),
            gpio: TCA9555::new(gpio_addr.into(), i2c),
            adc: ADS1115::new(ads1115_addr.into(), ADS1115Config::default(), i2c),
            coupler_initialized: false,
            mode: Mode::StandBy,
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

    pub async fn read_coupler_metrics(&mut self) -> Result<CouplerMetrics, &'static str> {
        if !self.coupler_initialized {
            self.adc
                .init()
                .await
                .map_err(|_| "Failed to init ADS1115 on LPF")?;
            self.coupler_initialized = true;
        }

        let forward_raw = self
            .adc
            .read_ain0()
            .await
            .map_err(|_| "Failed to read forward voltage")?;
        let reflected_raw = self
            .adc
            .read_ain1()
            .await
            .map_err(|_| "Failed to read reflected voltage")?;

        let forward_v = forward_raw as f32 * COUPLER_V_PER_COUNT;
        let reflected_v = reflected_raw as f32 * COUPLER_V_PER_COUNT;

        let forward_w = (forward_v * forward_v) * COUPLER_W_PER_V_SQUARED;
        let reflected_w = (reflected_v * reflected_v) * COUPLER_W_PER_V_SQUARED;

        let gamma = if forward_v.abs() > f32::EPSILON {
            (reflected_v.abs() / forward_v.abs()).min(0.999)
        } else {
            0.0
        };
        let vswr = if gamma >= 0.999 {
            f32::INFINITY
        } else {
            (1.0 + gamma) / (1.0 - gamma)
        };

        Ok(CouplerMetrics {
            forward_w,
            reflected_w,
            vswr,
        })
    }

    async fn initialize(&mut self) -> Result<(), &'static str> {
        self.gpio
            .init()
            .await
            .map_err(|_| "Failed to initialize TCA9555 for LPF")?;

        self.gpio
            .set_port_direction(TcaPort::Port0, 0x00)
            .await
            .map_err(|_| "Failed to set LPF Port0 direction")?;
        self.gpio
            .set_port_direction(TcaPort::Port1, 0x00)
            .await
            .map_err(|_| "Failed to set LPF Port1 direction")?;

        self.gpio
            .write_port(TcaPort::Port0, 0x00)
            .await
            .map_err(|_| "Failed to clear LPF Port0")?;
        self.gpio
            .write_port(TcaPort::Port1, 0x00)
            .await
            .map_err(|_| "Failed to clear LPF Port1")?;
        Ok(())
    }

    pub async fn update_state(&mut self) -> Result<(), &'static str> {
        if self.mode == Mode::StandBy {
            return Ok(());
        }
        if self.mode == Mode::WarmUp {
            self.initialize().await?;
        }

        let mut port0 = 0u8;
        let mut port1 = 0u8;

        let (filter_port, filter_pin) = self.config.find_filter(self.frequency);
        (port0, port1) = self
            .gpio
            .set_pin_value(port0, port1, filter_port, filter_pin, true);

        if self.mode == Mode::Tx {
            (port0, port1) = self.gpio.set_pin_value(
                port0,
                port1,
                self.config.tx_pin.port,
                self.config.tx_pin.pin,
                true,
            );
        }

        self.write_ports(port0, port1).await
    }

    async fn write_ports(&mut self, port0: u8, port1: u8) -> Result<(), &'static str> {
        self.gpio
            .write_port(TcaPort::Port0, port0)
            .await
            .map_err(|_| "Failed to write LPF Port0")?;
        self.gpio
            .write_port(TcaPort::Port1, port1)
            .await
            .map_err(|_| "Failed to write LPF Port1")
    }
}
