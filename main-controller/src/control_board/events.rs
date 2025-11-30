use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, signal::Signal};

/// Telemetry snapshot from the power buses (milli-volts / milli-amps).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PowerTelemetry {
    pub bus_13v8_voltage_mv: u16,
    pub bus_13v8_current_ma: u16,
    pub bus_3v3_voltage_mv: u16,
    pub bus_3v3_current_ma: u16,
    pub bus_vcc_voltage_mv: u16,
    pub bus_vcc_current_ma: u16,
}

pub static POWER_TELEMETRY: Signal<ThreadModeRawMutex, PowerTelemetry> = Signal::new();
