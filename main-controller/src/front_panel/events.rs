use common::protocol_types::MenuCommand;
use embassy_sync::channel::Channel;
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, watch::Watch};

use crate::{
    consts::AUDIO_BUFFER_SIZE,
    front_panel::types::{ButtonFunction, EncoderFunction},
};

pub static HEADPHONES_CONNECTED: Watch<ThreadModeRawMutex, bool, 2> = Watch::new();

#[derive(Debug, Clone, Copy)]
pub enum ButtonEvent {
    Press(ButtonFunction),
    Release(ButtonFunction),
}

#[derive(Debug, Clone, Copy)]
pub struct EncoderEvent {
    pub function: EncoderFunction,
    pub delta: i8,
}

pub static BUTTON_EVENTS: Channel<ThreadModeRawMutex, ButtonEvent, 16> = Channel::new();

pub static ENCODER_EVENTS: Channel<ThreadModeRawMutex, EncoderEvent, 16> = Channel::new();

pub static MENU_ENCODER_EVENTS: Channel<ThreadModeRawMutex, i8, 16> = Channel::new();

pub static AUDIO_MIC_BUFFER: Watch<ThreadModeRawMutex, [u16; AUDIO_BUFFER_SIZE], 2> = Watch::new();

pub static MENU_CMD_QUEUE: Channel<ThreadModeRawMutex, MenuCommand, 8> = Channel::new();
