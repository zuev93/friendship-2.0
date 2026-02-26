use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;

use crate::constants::OUTPUT_EVENTS_QUEUE_SIZE;

pub use druzhba_common::protocol_types::{
    ButtonEvent, ButtonState, EncoderDirection, EncoderEvent, HeadphonesEvent,
};

#[derive(Debug, Clone, Copy)]
pub enum OutputEvent {
    Button(ButtonEvent),
    Encoder(EncoderEvent),

    Headphones(HeadphonesEvent),
}

pub static OUTPUT_EVENTS: Channel<ThreadModeRawMutex, OutputEvent, OUTPUT_EVENTS_QUEUE_SIZE> =
    Channel::new();
