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
    IcomUpDown,   // Button 9 - ICOM Up/Down
    Cancel,       // Button 10 - Cancel/Back
    Ok,           // Button 11 - OK/Select
}

// Physical potentiometer mapping to logical functions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PotentiometerFunction {
    Volume,     // Audio volume control
    Microphone, // Microphone gain
    RfPower,    // RF transmit power
    IfGain,     // IF gain control
    Clarifier,  // RIT/XIT clarifier
    Squelch,    // Squelch level
}

pub type ControlBusType = &'static Mutex<ThreadModeRawMutex, ControlBus>;
