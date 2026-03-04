use crate::app::types::{Frequency, Mode, RfGainMode};
use crate::i2c_map::I2cAddress;
use crate::peripherals::types::{PeripherialI2c, PeripherialI2cMutex};
use common::drivers::pca9534::{Pin as PcaPin, PCA9534};
use common::drivers::tca9555::{Pin as TcaPin, Port as TcaPort, TCA9555};

#[derive(Clone, Copy)]
struct FilterLine {
    freq_min: Frequency,
    freq_max: Frequency,
    driver: FilterDriver,
}

#[derive(Clone, Copy)]
enum FilterDriver {
    Tca { port: TcaPort, pin: TcaPin },
    Pca { pin: PcaPin },
}

#[derive(Clone, Copy)]
struct BpfControlPins {
    att_on: FilterDriver,
    att_off: FilterDriver,
    tx: FilterDriver,
    rf_amp_on: FilterDriver,
    rf_amp_off: FilterDriver,
}

#[derive(Clone, Copy)]
struct BpfConfig {
    hpf_filters: [FilterLine; 10],
    bpf_filters: [FilterLine; 8],
    control: BpfControlPins,
}

impl BpfConfig {
    fn default() -> Self {
        Self {
            hpf_filters: [
                FilterLine {
                    freq_min: 1_000_000,
                    freq_max: 2_475_000,
                    driver: FilterDriver::Tca {
                        port: TcaPort::Port1,
                        pin: TcaPin::Pin1,
                    },
                },
                FilterLine {
                    freq_min: 2_475_000,
                    freq_max: 4_860_000,
                    driver: FilterDriver::Tca {
                        port: TcaPort::Port0,
                        pin: TcaPin::Pin1,
                    },
                },
                FilterLine {
                    freq_min: 4_860_000,
                    freq_max: 7_740_000,
                    driver: FilterDriver::Tca {
                        port: TcaPort::Port0,
                        pin: TcaPin::Pin2,
                    },
                },
                FilterLine {
                    freq_min: 7_740_000,
                    freq_max: 11_500_000,
                    driver: FilterDriver::Tca {
                        port: TcaPort::Port0,
                        pin: TcaPin::Pin4,
                    },
                },
                FilterLine {
                    freq_min: 11_500_000,
                    freq_max: 15_700_000,
                    driver: FilterDriver::Pca { pin: PcaPin::Pin2 },
                },
                FilterLine {
                    freq_min: 15_700_000,
                    freq_max: 19_265_000,
                    driver: FilterDriver::Pca { pin: PcaPin::Pin1 },
                },
                FilterLine {
                    freq_min: 19_265_000,
                    freq_max: 22_706_000,
                    driver: FilterDriver::Tca {
                        port: TcaPort::Port0,
                        pin: TcaPin::Pin7,
                    },
                },
                FilterLine {
                    freq_min: 22_706_000,
                    freq_max: 26_038_000,
                    driver: FilterDriver::Pca { pin: PcaPin::Pin4 },
                },
                FilterLine {
                    freq_min: 26_038_000,
                    freq_max: 50_000_000,
                    driver: FilterDriver::Tca {
                        port: TcaPort::Port0,
                        pin: TcaPin::Pin5,
                    },
                },
                FilterLine {
                    freq_min: 0,
                    freq_max: 0,
                    driver: FilterDriver::Tca {
                        port: TcaPort::Port1,
                        pin: TcaPin::Pin7,
                    },
                },
            ],
            bpf_filters: [
                FilterLine {
                    freq_min: 1_300_000,
                    freq_max: 2_500_000,
                    driver: FilterDriver::Tca {
                        port: TcaPort::Port0,
                        pin: TcaPin::Pin6,
                    },
                },
                FilterLine {
                    freq_min: 3_100_000,
                    freq_max: 4_000_000,
                    driver: FilterDriver::Pca { pin: PcaPin::Pin0 },
                },
                FilterLine {
                    freq_min: 6_650_000,
                    freq_max: 7_550_000,
                    driver: FilterDriver::Pca { pin: PcaPin::Pin3 },
                },
                FilterLine {
                    freq_min: 9_000_000,
                    freq_max: 11_000_000,
                    driver: FilterDriver::Tca {
                        port: TcaPort::Port0,
                        pin: TcaPin::Pin3,
                    },
                },
                FilterLine {
                    freq_min: 13_800_000,
                    freq_max: 14_750_000,
                    driver: FilterDriver::Tca {
                        port: TcaPort::Port0,
                        pin: TcaPin::Pin0,
                    },
                },
                FilterLine {
                    freq_min: 17_200_000,
                    freq_max: 19_200_000,
                    driver: FilterDriver::Tca {
                        port: TcaPort::Port1,
                        pin: TcaPin::Pin2,
                    },
                },
                FilterLine {
                    freq_min: 20_500_000,
                    freq_max: 21_950_000,
                    driver: FilterDriver::Tca {
                        port: TcaPort::Port1,
                        pin: TcaPin::Pin5,
                    },
                },
                FilterLine {
                    freq_min: 0,
                    freq_max: 0,
                    driver: FilterDriver::Tca {
                        port: TcaPort::Port1,
                        pin: TcaPin::Pin4,
                    },
                },
            ],
            control: BpfControlPins {
                att_on: FilterDriver::Pca { pin: PcaPin::Pin5 },
                att_off: FilterDriver::Tca {
                    port: TcaPort::Port1,
                    pin: TcaPin::Pin0,
                },
                tx: FilterDriver::Pca { pin: PcaPin::Pin6 },
                rf_amp_on: FilterDriver::Tca {
                    port: TcaPort::Port1,
                    pin: TcaPin::Pin3,
                },
                rf_amp_off: FilterDriver::Tca {
                    port: TcaPort::Port1,
                    pin: TcaPin::Pin6,
                },
            },
        }
    }

    fn find_hpf(&self, frequency: Frequency) -> FilterDriver {
        Self::pick_filter(&self.hpf_filters, frequency)
    }

    fn find_bpf(&self, frequency: Frequency) -> FilterDriver {
        Self::pick_filter(&self.bpf_filters, frequency)
    }

    fn pick_filter(filters: &[FilterLine], frequency: Frequency) -> FilterDriver {
        filters
            .iter()
            .find(|filter| frequency >= filter.freq_min && frequency <= filter.freq_max)
            .unwrap_or_else(|| filters.last().expect("filters array is not empty"))
            .driver
    }
}

pub struct Bpf {
    config: BpfConfig,
    tca: TCA9555<PeripherialI2c>,
    pca: PCA9534<PeripherialI2c>,
    mode: Mode,
    rf_gain_mode: RfGainMode,
    frequency: Frequency,
}

impl Bpf {
    pub fn new(
        i2c: PeripherialI2cMutex,
        tca_addr: I2cAddress,
        pca_addr: I2cAddress,
    ) -> Self {
        Self {
            config: BpfConfig::default(),
            tca: TCA9555::new(tca_addr.into(), i2c),
            pca: PCA9534::new(pca_addr.into(), i2c),
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

    async fn initialize(&mut self) -> Result<(), &'static str> {
        {
            self.tca
                .init()
                .await
                .map_err(|_| "Failed to initialize BPF TCA9555")?;

            self.tca
                .set_port_direction(TcaPort::Port0, 0x00)
                .await
                .map_err(|_| "Failed to set BPF Port0 direction")?;
            self.tca
                .set_port_direction(TcaPort::Port1, 0x00)
                .await
                .map_err(|_| "Failed to set BPF Port1 direction")?;

            self.tca
                .write_port(TcaPort::Port0, 0x00)
                .await
                .map_err(|_| "Failed to clear BPF Port0")?;
            self.tca
                .write_port(TcaPort::Port1, 0x00)
                .await
                .map_err(|_| "Failed to clear BPF Port1")?;
        }

        self.pca
            .init()
            .await
            .map_err(|_| "Failed to initialize BPF PCA9534")?;
        self.pca
            .set_direction(0x00)
            .await
            .map_err(|_| "Failed to set BPF PCA9534 direction")?;
        self.pca
            .write_port(0x00)
            .await
            .map_err(|_| "Failed to clear BPF PCA9534 port")?;

        Ok(())
    }

    fn apply_filter_driver(
        &self,
        driver: &FilterDriver,
        port0: u8,
        port1: u8,
        pca_port: u8,
    ) -> (u8, u8, u8) {
        self.apply_control_pin(driver, true, port0, port1, pca_port)
    }

    fn apply_control_pin(
        &self,
        driver: &FilterDriver,
        state: bool,
        port0: u8,
        port1: u8,
        pca_port: u8,
    ) -> (u8, u8, u8) {
        match driver {
            FilterDriver::Tca { port, pin } => {
                let (p0, p1) = self.tca.set_pin_value(port0, port1, *port, *pin, state);
                (p0, p1, pca_port)
            }
            FilterDriver::Pca { pin } => {
                let p = self.pca.set_pin_value(pca_port, *pin, state);
                (port0, port1, p)
            }
        }
    }

    pub async fn update_state(&mut self) -> Result<(), &'static str> {
        match self.mode {
            Mode::StandBy => {
                self.write_tca_ports(0x00, 0x00).await?;
                self.pca
                    .write_port(0x00)
                    .await
                    .map_err(|_| "Failed to turn off BPF PCA9534")?;
                return Ok(());
            }
            Mode::WarmUp => {
                self.initialize().await?;
                return Ok(());
            }
            _ => {}
        }

        let mut tca_port0: u8 = 0;
        let mut tca_port1: u8 = 0;
        let mut pca_port: u8 = 0;

        // Select HPF and BPF based on current frequency
        let hpf_driver = self.config.find_hpf(self.frequency);
        (tca_port0, tca_port1, pca_port) =
            self.apply_filter_driver(&hpf_driver, tca_port0, tca_port1, pca_port);

        let bpf_driver = self.config.find_bpf(self.frequency);
        (tca_port0, tca_port1, pca_port) =
            self.apply_filter_driver(&bpf_driver, tca_port0, tca_port1, pca_port);

        let tx_active = self.mode == Mode::Tx;
        (tca_port0, tca_port1, pca_port) =
            self.apply_control_pin(&self.config.control.tx, tx_active, tca_port0, tca_port1, pca_port);

        let att_on = self.rf_gain_mode == RfGainMode::Attenuator;
        (tca_port0, tca_port1, pca_port) =
            self.apply_control_pin(&self.config.control.att_on, att_on, tca_port0, tca_port1, pca_port);
        (tca_port0, tca_port1, pca_port) =
            self.apply_control_pin(&self.config.control.att_off, !att_on, tca_port0, tca_port1, pca_port);

        let rf_amp_on = self.rf_gain_mode == RfGainMode::RfDouble;
        (tca_port0, tca_port1, pca_port) =
            self.apply_control_pin(&self.config.control.rf_amp_on, rf_amp_on, tca_port0, tca_port1, pca_port);
        (tca_port0, tca_port1, pca_port) =
            self.apply_control_pin(&self.config.control.rf_amp_off, !rf_amp_on, tca_port0, tca_port1, pca_port);

        self.write_tca_ports(tca_port0, tca_port1).await?;
        self.pca
            .write_port(pca_port)
            .await
            .map_err(|_| "Failed to write BPF PCA9534")?;

        Ok(())
    }

    async fn write_tca_ports(&mut self, port0: u8, port1: u8) -> Result<(), &'static str> {
        self.tca
            .write_port(TcaPort::Port0, port0)
            .await
            .map_err(|_| "Failed to write BPF TCA9555 Port0")?;
        self.tca
            .write_port(TcaPort::Port1, port1)
            .await
            .map_err(|_| "Failed to write BPF TCA9555 Port1")
    }
}
