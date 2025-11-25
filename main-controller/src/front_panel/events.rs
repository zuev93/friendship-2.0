use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;

use crate::front_panel::types::{ButtonFunction, PotentiometerFunction};

// Button events
#[derive(Debug, Clone, Copy)]
pub enum ButtonEvent {
    Press(ButtonFunction),
    Release(ButtonFunction),
}

// Potentiometer event
#[derive(Debug, Clone, Copy)]
pub struct PotentiometerEvent {
    pub function: PotentiometerFunction,
    pub value: i16,
}

#[derive(Debug, Clone, Copy)]
pub struct BandEncoderRotateEvent {
    pub delta: i8, // Positive for clockwise, negative for counter-clockwise
}

#[derive(Debug, Clone, Copy)]
pub struct VfoEncoderRotateEvent {
    pub delta: i8, // Positive for clockwise, negative for counter-clockwise
}

pub static BUTTON_EVENTS: Channel<ThreadModeRawMutex, ButtonEvent, 16> = Channel::new();

pub static POTENTIOMETER_EVENTS: Channel<ThreadModeRawMutex, PotentiometerEvent, 16> =
    Channel::new();

pub static BAND_ENCODER_EVENTS: Channel<ThreadModeRawMutex, BandEncoderRotateEvent, 16> =
    Channel::new();

pub static VFO_ENCODER_EVENTS: Channel<ThreadModeRawMutex, VfoEncoderRotateEvent, 16> =
    Channel::new();
