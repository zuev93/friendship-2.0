use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, signal::Signal};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PowerTelemetry {
    pub vbus_voltage_mv: u32,
    pub vbus_current_ma: i32,
    pub vbus_power_mw: u32,
    pub pa_voltage_mv: u32,
    pub pa_current_ma: i32,
    pub pa_power_mw: u32,
    pub rail_3v3_voltage_mv: u32,
    pub rail_3v3_current_ma: i32,
    pub rail_3v3_power_mw: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PdContract {
    pub voltage_mv: u32,
    pub current_ma: u32,
    pub power_mw: u32,
}

impl Default for PdContract {
    fn default() -> Self {
        Self {
            voltage_mv: 5000,
            current_ma: 900,
            power_mw: 4500,
        }
    }
}

pub static POWER_TELEMETRY: Signal<ThreadModeRawMutex, PowerTelemetry> = Signal::new();
pub static PD_CONTRACT: Signal<ThreadModeRawMutex, PdContract> = Signal::new();
