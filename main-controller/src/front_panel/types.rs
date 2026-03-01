use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};

use crate::front_panel::modules::control_bus::ControlBus;

// Physical button mapping to logical functions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ButtonFunction {
    Power,        // Button 0 - StandBy / On
    Transmit,     // Button 1 - PTT
    Tone,         // Button 2 - Tone / No Tone
    TransmitMode, // Button 3 - cycles SSB -> CW -> AM
    Rit,          // Button 4 - toggles RIT on/off
    RfGain,       // Button 5 - cycles Attenuator -> Normal -> RF Amp
    Agc,          // Button 6 - toggles AGC on/off
    IcomPtt,      // Button 7 - ICOM PTT
    IcomSql,      // Button 8 - ICOM Squelch
    NoiseBlanker, // Noise blanker on/off
    IcomUpDown,   // Button 9 - ICOM Up/Down
    Filter,       // Cycles crystal filter: Single -> DoubleWide -> DoubleNarrow
    Cancel,       // Button 10 - Cancel/Back
    Ok,           // Button 11 - OK/Select
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EncoderFunction {
    Band,
    Vfo,
    Volume,
    RfPower,
    Microphone,
    IfGain,
    Clarifier,
    Squelch,
    NbLevel,
    Menu,
}

pub type ControlBusType = &'static Mutex<ThreadModeRawMutex, ControlBus>;
