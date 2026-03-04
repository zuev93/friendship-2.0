/*
 * LPF (Low Pass Filter) Module
 * Controls low-pass filters (TCA9555) and reads coupler power/VSWR (ADS1115) on the same board.
 * TCA9555 pins are routed through ULN2002 Darlington drivers to relay coils.
 * Pin mapping is non-linear due to PCB routing through two ULN2002 chips (U4, U6).
 * ADS1115: AIN0 = forward voltage (AD8307 log detector), AIN1 = reflected voltage (AD8307 log detector)
 */

use crate::app::cordic_math::{with_cordic, CordicMutex};
use crate::app::types::{CouplerMetrics, Frequency, Mode};
use crate::i2c_map::I2cAddress;
use crate::peripherals::types::{PeripherialI2c, PeripherialI2cMutex};
use common::drivers::ads1115::{ADS1115Config, ADS1115};
use common::drivers::tca9555::{Pin as TcaPin, Port as TcaPort, TCA9555};

const ADC_V_PER_COUNT: f32 = 4.096 / 32768.0;

const AD8307_SLOPE_V_PER_DB: f32 = 0.025;
const AD8307_INTERCEPT_DBM: f32 = -84.0;
const COUPLER_DB: f32 = 20.0;

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
                    pin: TcaPin::Pin7,
                },
                LpfFilter {
                    freq_min: 5_000_000,
                    freq_max: 7_500_000,
                    port: TcaPort::Port0,
                    pin: TcaPin::Pin1,
                },
                LpfFilter {
                    freq_min: 10_000_000,
                    freq_max: 10_150_000,
                    port: TcaPort::Port0,
                    pin: TcaPin::Pin2,
                },
                LpfFilter {
                    freq_min: 14_000_000,
                    freq_max: 14_350_000,
                    port: TcaPort::Port1,
                    pin: TcaPin::Pin1,
                },
                LpfFilter {
                    freq_min: 18_000_000,
                    freq_max: 18_168_000,
                    port: TcaPort::Port1,
                    pin: TcaPin::Pin0,
                },
                LpfFilter {
                    freq_min: 21_000_000,
                    freq_max: 21_450_000,
                    port: TcaPort::Port0,
                    pin: TcaPin::Pin3,
                },
                LpfFilter {
                    freq_min: 24_000_000,
                    freq_max: 30_000_000,
                    port: TcaPort::Port0,
                    pin: TcaPin::Pin5,
                },
                LpfFilter {
                    freq_min: 50_000_000,
                    freq_max: 54_000_000,
                    port: TcaPort::Port0,
                    pin: TcaPin::Pin6,
                },
            ],
            tx_pin: LpfFilter {
                freq_min: 0,
                freq_max: 0,
                port: TcaPort::Port0,
                pin: TcaPin::Pin4,
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
    cordic: &'static CordicMutex,
}

impl Lpf {
    pub fn new(
        i2c: PeripherialI2cMutex,
        gpio_addr: I2cAddress,
        ads1115_addr: I2cAddress,
        cordic: &'static CordicMutex,
    ) -> Self {
        Self {
            config: LpfConfig::default(),
            gpio: TCA9555::new(gpio_addr.into(), i2c),
            adc: ADS1115::new(ads1115_addr.into(), ADS1115Config::default(), i2c),
            coupler_initialized: false,
            mode: Mode::StandBy,
            frequency: 0,
            cordic,
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

        let forward_v = forward_raw as f32 * ADC_V_PER_COUNT;
        let reflected_v = reflected_raw as f32 * ADC_V_PER_COUNT;

        let forward_dbm = ad8307_voltage_to_dbm(forward_v) + COUPLER_DB;
        let reflected_dbm = ad8307_voltage_to_dbm(reflected_v) + COUPLER_DB;

        let forward_w = self.dbm_to_watts(forward_dbm);
        let reflected_w = self.dbm_to_watts(reflected_dbm);

        let return_loss_db = forward_dbm - reflected_dbm;
        let gamma = with_cordic(self.cordic, |c| c.pow10f(-return_loss_db / 20.0)).min(0.999);

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

    fn dbm_to_watts(&self, dbm: f32) -> f32 {
        with_cordic(self.cordic, |c| c.pow10f((dbm - 30.0) / 10.0))
    }
}

fn ad8307_voltage_to_dbm(v: f32) -> f32 {
    v / AD8307_SLOPE_V_PER_DB + AD8307_INTERCEPT_DBM
}
