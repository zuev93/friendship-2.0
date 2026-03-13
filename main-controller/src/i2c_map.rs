#[derive(Debug, Clone, Copy)]
pub struct I2cAddress(u8);

impl I2cAddress {
    pub const fn new(addr: u8) -> Self {
        Self(addr)
    }

    pub fn into_raw(self) -> u8 {
        self.0
    }
}

impl From<I2cAddress> for u8 {
    fn from(addr: I2cAddress) -> Self {
        addr.into_raw()
    }
}

pub struct MainI2cMap {
    pub si5351: I2cAddress,
    pub filter_pca9536: I2cAddress,
    pub if_amp_mcp4728: I2cAddress,
    pub if_amp_ads1015_rssi: I2cAddress,
    pub audio_cs4272: I2cAddress,
    pub audio_panel_pca9534: I2cAddress,
    pub filter_nb_mcp4725: I2cAddress,
}

impl MainI2cMap {
    pub const fn new() -> Self {
        Self {
            si5351: I2cAddress::new(0x60),
            filter_pca9536: I2cAddress::new(0x41),
            if_amp_mcp4728: I2cAddress::new(0x61),
            if_amp_ads1015_rssi: I2cAddress::new(0b1001000),
            audio_cs4272: I2cAddress::new(0x10),
            audio_panel_pca9534: I2cAddress::new(0b0100011),
            filter_nb_mcp4725: I2cAddress::new(0x62),
        }
    }
}

pub struct PeripheralI2cMap {
    pub bpf_tca_gpio: I2cAddress,
    pub bpf_pca_gpio: I2cAddress,
    pub lpf_gpio: I2cAddress,
    pub lpf_ads1115: I2cAddress,
    pub hf_amp_driver_dac: I2cAddress,
    pub hf_amp_final_dac: I2cAddress,
    pub hf_amp_adc: I2cAddress,
}

impl PeripheralI2cMap {
    pub const fn new() -> Self {
        Self {
            bpf_tca_gpio: I2cAddress::new(0x21),
            bpf_pca_gpio: I2cAddress::new(0x23),
            lpf_gpio: I2cAddress::new(0x20),
            lpf_ads1115: I2cAddress::new(0x48),
            hf_amp_driver_dac: I2cAddress::new(0x60),
            hf_amp_final_dac: I2cAddress::new(0x61),
            hf_amp_adc: I2cAddress::new(0x49),
        }
    }
}

pub struct ControlBoardI2cMap {
    pub ina228_vbus: I2cAddress,
    pub ina228_pa: I2cAddress,
    pub ina228_3v3: I2cAddress,
}

impl ControlBoardI2cMap {
    pub const fn new() -> Self {
        Self {
            ina228_vbus: I2cAddress::new(0x41),
            ina228_pa: I2cAddress::new(0x40),
            ina228_3v3: I2cAddress::new(0x44),
        }
    }
}

pub struct I2cMap {
    pub main: MainI2cMap,
    pub peripherals: PeripheralI2cMap,
    pub control_board: ControlBoardI2cMap,
}

impl I2cMap {
    pub const fn new() -> Self {
        Self {
            main: MainI2cMap::new(),
            peripherals: PeripheralI2cMap::new(),
            control_board: ControlBoardI2cMap::new(),
        }
    }
}
