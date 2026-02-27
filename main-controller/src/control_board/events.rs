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

pub static POWER_TELEMETRY: Signal<ThreadModeRawMutex, PowerTelemetry> = Signal::new();
