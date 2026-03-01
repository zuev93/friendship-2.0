use embassy_stm32::gpio::{Level, Output, Pin, Speed};
use embassy_stm32::Peri;
use embassy_time::Timer;

use crate::{
    app::types::Mode,
    control_board::{
        events::{EmergencyReason, PdContract, PowerTelemetry, RAIL_50V_READY},
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
    pin_50v_enabled: Output<'static>,
    pin_50v_mode: Output<'static>,
    pin_3v3_enabled: Output<'static>,
    ina_vbus: Ina228<ControlBoardI2C>,
    ina_pa: Ina228<ControlBoardI2C>,
    ina_3v3: Ina228<ControlBoardI2C>,
    mode: Mode,
}

impl PowerControl {
    pub fn new(
        pin_50v_enabled: Peri<'static, impl Pin>,
        pin_50v_mode: Peri<'static, impl Pin>,
        pin_3v3_enabled: Peri<'static, impl Pin>,
        i2c: ControlBoardI2cMutex,
        ina228_vbus_addr: I2cAddress,
        ina228_pa_addr: I2cAddress,
        ina228_3v3_addr: I2cAddress,
    ) -> Self {
        Self {
            pin_50v_enabled: Output::new(pin_50v_enabled, Level::Low, Speed::Medium),
            pin_50v_mode: Output::new(pin_50v_mode, Level::Low, Speed::Medium),
            pin_3v3_enabled: Output::new(pin_3v3_enabled, Level::Low, Speed::Medium),
            mode: Mode::StandBy,
            ina_vbus: Ina228::new(ina228_vbus_addr.into(), SHUNT_MOHM, VBUS_MAX_CURRENT_MA, i2c),
            ina_pa: Ina228::new(ina228_pa_addr.into(), SHUNT_MOHM, PA_MAX_CURRENT_MA, i2c),
            ina_3v3: Ina228::new(ina228_3v3_addr.into(), SHUNT_MOHM, RAIL_3V3_MAX_CURRENT_MA, i2c),
        }
    }

    pub async fn set_mode(&mut self, mode: Mode) -> Result<(), &'static str> {
        let prev = self.mode;
        self.mode = mode;

        if mode == Mode::WarmUp {
            self.init().await?;
        }

        match mode {
            Mode::StandBy => {
                self.pin_50v_mode.set_low();
                self.pin_50v_enabled.set_low();
                self.pin_3v3_enabled.set_low();
                RAIL_50V_READY.sender().send(false);
            }
            Mode::WarmUp => {
                self.pin_50v_mode.set_low();
                self.pin_50v_enabled.set_low();
                self.pin_3v3_enabled.set_high();
                RAIL_50V_READY.sender().send(false);
            }
            Mode::Rx => {
                if prev == Mode::Tx {
                    self.pin_50v_mode.set_low();
                    RAIL_50V_READY.sender().send(false);
                }
                self.pin_50v_enabled.set_high();
                self.pin_3v3_enabled.set_high();
            }
            Mode::Tx => {
                self.pin_50v_enabled.set_high();
                self.pin_3v3_enabled.set_high();
                self.pin_50v_mode.set_high();
                Timer::after_millis(50).await;
                RAIL_50V_READY.sender().send(true);
            }
        }

        Ok(())
    }

    pub async fn read_pa_current_ma(&mut self) -> Result<i32, &'static str> {
        self.ina_pa
            .read_current_ma()
            .await
            .map_err(|_| "INA228 PA current read failed")
    }

    pub async fn set_pa_fast_mode(&mut self) -> Result<(), &'static str> {
        self.ina_pa
            .set_fast_mode()
            .await
            .map_err(|_| "INA228 PA set fast mode failed")
    }

    pub async fn set_pa_normal_mode(&mut self) -> Result<(), &'static str> {
        self.ina_pa
            .set_normal_mode()
            .await
            .map_err(|_| "INA228 PA set normal mode failed")
    }

    pub fn check_limits(
        &self,
        telemetry: &PowerTelemetry,
        pd_contract: &PdContract,
    ) -> Option<EmergencyReason> {
        let vbus_limit = (pd_contract.current_ma * 95) / 100;
        if telemetry.vbus_current_ma > 0 && telemetry.vbus_current_ma as u32 > vbus_limit {
            return Some(EmergencyReason::VbusOvercurrent);
        }

        if self.mode != Mode::Tx && telemetry.pa_current_ma > 500 {
            return Some(EmergencyReason::PaOvercurrent);
        }

        None
    }

    pub fn emergency_shutdown(&mut self) {
        self.pin_50v_mode.set_low();
        self.pin_50v_enabled.set_low();
        self.mode = Mode::StandBy;
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
