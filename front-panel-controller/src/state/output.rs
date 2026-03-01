use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;

use crate::constants::OUTPUT_EVENTS_QUEUE_SIZE;

pub use druzhba_common::protocol_types::{
    ButtonEvent, ButtonState, DisplayFpsEvent, EncoderDirection, EncoderEvent, HeadphonesEvent,
};

#[derive(Debug, Clone, Copy)]
pub enum OutputEvent {
    Button(ButtonEvent),
    Encoder(EncoderEvent),
    Headphones(HeadphonesEvent),
    DisplayFps(DisplayFpsEvent),
}

pub static OUTPUT_EVENTS: Channel<ThreadModeRawMutex, OutputEvent, OUTPUT_EVENTS_QUEUE_SIZE> =
    Channel::new();
