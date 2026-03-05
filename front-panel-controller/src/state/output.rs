use druzhba_common::PlatformMutex;
use embassy_sync::channel::Channel;

pub use druzhba_common::protocol_types::{
    ButtonEvent, ButtonState, DisplayFpsEvent, EncoderDirection, EncoderEvent, HeadphonesEvent,
};

pub const OUTPUT_EVENTS_QUEUE_SIZE: usize = 32;

#[derive(Debug, Clone, Copy)]
pub enum OutputEvent {
    Button(ButtonEvent),
    Encoder(EncoderEvent),
    Headphones(HeadphonesEvent),
    DisplayFps(DisplayFpsEvent),
}

pub static OUTPUT_EVENTS: Channel<PlatformMutex, OutputEvent, OUTPUT_EVENTS_QUEUE_SIZE> =
    Channel::new();
