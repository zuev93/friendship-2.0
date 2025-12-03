use embassy_stm32::{
    gpio::{Level, Output, Pin, Speed},
    i2c::{self, mode as i2c_mode, I2c, SclPin, SdaPin},
    mode, Peri,
};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use static_cell::StaticCell;

use crate::{
    app::types::Mode,
    i2c_map::I2cAddress,
    control_board::{
        events::PowerTelemetry,
        types::ControlBoardI2C,
    },
};
use common::drivers::ina3221::{Ina3221, Ina3221Measurements};

const SHUNTS: [u16; 3] = [10, 12, 20];

/// Power rail controller: holds GPIO power switches + INA3221 telemetry reader.
pub struct PowerControl {
    pin_13v8_enabled: Output<'static>,
    pin_3v3_enabled: Output<'static>,
    ina3221: Ina3221<ControlBoardI2C>,
    mode: Mode,
}

impl PowerControl {
    pub fn new<T1: i2c::Instance>(
        pin_13v8_enabled: Peri<'static, impl Pin>,
        pin_3v3_enabled: Peri<'static, impl Pin>,
        i2c_periph: Peri<'static, T1>,
        sda: Peri<'static, impl SdaPin<T1>>,
        scl: Peri<'static, impl SclPin<T1>>,
        ina3221_addr: I2cAddress,
    ) -> Self {
        let mut i2c_config = i2c::Config::default();
        i2c_config.sda_pullup = true;
        i2c_config.scl_pullup = true;

        let i2c = I2c::new_blocking(i2c_periph, scl, sda, i2c_config);
        static STATIC_CELL: StaticCell<
            Mutex<ThreadModeRawMutex, I2c<'static, mode::Blocking, i2c_mode::Master>>,
        > = StaticCell::new();
        let i2c_mutex = STATIC_CELL.init(Mutex::new(i2c));

        Self {
            pin_13v8_enabled: Output::new(pin_13v8_enabled, Level::Low, Speed::Medium),
            pin_3v3_enabled: Output::new(pin_3v3_enabled, Level::Low, Speed::Medium),
            mode: Mode::StandBy,
            ina3221: Ina3221::new(ina3221_addr.into(), SHUNTS, i2c_mutex),
        }
    }

    pub async fn set_mode(&mut self, mode: Mode) -> Result<(), &'static str> {
        self.mode = mode;

        self.update_state().await
    }

    async fn update_state(&mut self) -> Result<(), &'static str> {
        if self.mode == Mode::WarmUp {
            self.init().await?;
        }
        let level = if self.mode != Mode::StandBy {
            Level::High
        } else {
            Level::Low
        };
        self.pin_13v8_enabled.set_level(level);
        self.pin_3v3_enabled.set_level(level);
        Ok(())
    }

    /// Read telemetry from INA3221 (once driver is wired).
    pub async fn read_power_telemetry(&mut self) -> Result<PowerTelemetry, &'static str> {
        let data = self
            .ina3221
            .read_all()
            .await
            .map_err(|_| "INA3221 read failed")?;
        return Ok(telemetry_from_ina(&data));
    }

    pub async fn init(&mut self) -> Result<(), &'static str> {
        self.ina3221.init().await.map_err(|_| "INA3221 init failed")
    }
}

fn telemetry_from_ina(data: &Ina3221Measurements) -> PowerTelemetry {
    PowerTelemetry {
        bus_13v8_voltage_mv: data.ch2.bus_voltage_mv,
        bus_13v8_current_ma: data.ch2.shunt_current_ma.max(0) as u16,
        bus_3v3_voltage_mv: data.ch1.bus_voltage_mv,
        bus_3v3_current_ma: data.ch1.shunt_current_ma.max(0) as u16,
        bus_vcc_voltage_mv: data.ch3.bus_voltage_mv,
        bus_vcc_current_ma: data.ch3.shunt_current_ma.max(0) as u16,
    }
}
