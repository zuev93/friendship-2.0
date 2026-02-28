use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;

#[derive(Clone, Copy, PartialEq)]
pub enum BsodError {
    FrontPanelInitFailed,
    DisplayInitFailed,
    MainBoardInitFailed,
}

pub static BSOD: Signal<ThreadModeRawMutex, BsodError> = Signal::new();

pub static ERROR_MESSAGES: Channel<ThreadModeRawMutex, &'static str, 16> = Channel::new();

pub async fn error(message: &'static str) {
    ERROR_MESSAGES.send(message).await;
}
