use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, signal::Signal};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmergencyReason {
    VbusOvercurrent,
    PaOvercurrent,
    Thermal,
}

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

impl PowerTelemetry {
    pub fn power_budget(&self, contract: &PdContract) -> i32 {
        let max_current = contract.current_ma as i32;
        if max_current <= 0 {
            return -10000;
        }
        let current = self.vbus_current_ma.max(0) as i32;
        let headroom = max_current - current;
        (headroom * 10000 / max_current).clamp(-10000, 10000)
    }
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
pub static PA_CURRENT_REQUEST: Signal<ThreadModeRawMutex, ()> = Signal::new();
pub static PA_CURRENT_READING: Signal<ThreadModeRawMutex, i32> = Signal::new();
pub static PA_FAST_MODE: Signal<ThreadModeRawMutex, bool> = Signal::new();
pub static RAIL_50V_READY: Signal<ThreadModeRawMutex, bool> = Signal::new();
pub static EMERGENCY_SHUTDOWN: Signal<ThreadModeRawMutex, EmergencyReason> = Signal::new();
