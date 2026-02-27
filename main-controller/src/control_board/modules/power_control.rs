use embassy_stm32::gpio::{Level, Output, Pin, Speed};
use embassy_stm32::Peri;

use crate::{
    app::types::Mode,
    control_board::{
        events::PowerTelemetry,
        types::{ControlBoardI2C, ControlBoardI2cMutex},
    },
    i2c_map::I2cAddress,
};
use common::drivers::ina228::Ina228;

const SHUNT_MOHM: u16 = 10;
const VBUS_MAX_CURRENT_MA: u32 = 5000;
const PA_MAX_CURRENT_MA: u32 = 5000;
const RAIL_3V3_MAX_CURRENT_MA: u32 = 2000;

pub struct PowerControl {
    pin_13v8_enabled: Output<'static>,
    pin_3v3_enabled: Output<'static>,
    ina_vbus: Ina228<ControlBoardI2C>,
    ina_pa: Ina228<ControlBoardI2C>,
    ina_3v3: Ina228<ControlBoardI2C>,
    mode: Mode,
}

impl PowerControl {
    pub fn new(
        pin_13v8_enabled: Peri<'static, impl Pin>,
        pin_3v3_enabled: Peri<'static, impl Pin>,
        i2c: ControlBoardI2cMutex,
        ina228_vbus_addr: I2cAddress,
        ina228_pa_addr: I2cAddress,
        ina228_3v3_addr: I2cAddress,
    ) -> Self {
        Self {
            pin_13v8_enabled: Output::new(pin_13v8_enabled, Level::Low, Speed::Medium),
            pin_3v3_enabled: Output::new(pin_3v3_enabled, Level::Low, Speed::Medium),
            mode: Mode::StandBy,
            ina_vbus: Ina228::new(ina228_vbus_addr.into(), SHUNT_MOHM, VBUS_MAX_CURRENT_MA, i2c),
            ina_pa: Ina228::new(ina228_pa_addr.into(), SHUNT_MOHM, PA_MAX_CURRENT_MA, i2c),
            ina_3v3: Ina228::new(ina228_3v3_addr.into(), SHUNT_MOHM, RAIL_3V3_MAX_CURRENT_MA, i2c),
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

    pub async fn read_power_telemetry(&mut self) -> Result<PowerTelemetry, &'static str> {
        let vbus = self.ina_vbus.read_all().await.map_err(|_| "INA228 VBUS read failed")?;
        let pa = self.ina_pa.read_all().await.map_err(|_| "INA228 PA read failed")?;
        let rail = self.ina_3v3.read_all().await.map_err(|_| "INA228 3V3 read failed")?;

        Ok(PowerTelemetry {
            vbus_voltage_mv: vbus.bus_voltage_mv,
            vbus_current_ma: vbus.current_ma,
            vbus_power_mw: vbus.power_mw,
            pa_voltage_mv: pa.bus_voltage_mv,
            pa_current_ma: pa.current_ma,
            pa_power_mw: pa.power_mw,
            rail_3v3_voltage_mv: rail.bus_voltage_mv,
            rail_3v3_current_ma: rail.current_ma,
            rail_3v3_power_mw: rail.power_mw,
        })
    }

    pub async fn init(&mut self) -> Result<(), &'static str> {
        self.ina_vbus.init().await.map_err(|_| "INA228 VBUS init failed")?;
        self.ina_pa.init().await.map_err(|_| "INA228 PA init failed")?;
        self.ina_3v3.init().await.map_err(|_| "INA228 3V3 init failed")?;
        Ok(())
    }
}
