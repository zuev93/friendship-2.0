use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;

use crate::PlatformMutex;

#[derive(Clone, Copy, PartialEq)]
pub enum BsodError {
    FrontPanelInitFailed,
    DisplayInitFailed,
    MainBoardInitFailed,
    Crash,
}

pub static BSOD: Signal<PlatformMutex, BsodError> = Signal::new();

pub static ERROR_MESSAGES: Channel<PlatformMutex, &'static str, 16> = Channel::new();

pub async fn error(message: &'static str) {
    ERROR_MESSAGES.send(message).await;
}
