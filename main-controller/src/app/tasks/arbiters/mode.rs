use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::events::MODE;
use crate::app::types::Mode;

pub enum ModeCommand {
    PowerToggle,
    TransmitPress,
    TransmitRelease,
    TonePress,
    ToneRelease,
}

pub static MODE_CMD: Signal<ThreadModeRawMutex, ModeCommand> = Signal::new();

#[embassy_executor::task]
pub async fn mode_arbiter_task() {
    let mut mode = Mode::StandBy;

    loop {
        let cmd = MODE_CMD.wait().await;
        let new_mode = match cmd {
            ModeCommand::PowerToggle => {
                if mode == Mode::StandBy {
                    Mode::WarmUp
                } else {
                    Mode::StandBy
                }
            }
            ModeCommand::TransmitPress => {
                if mode == Mode::Rx {
                    Mode::Tx
                } else {
                    continue;
                }
            }
            ModeCommand::TransmitRelease => {
                if mode == Mode::Tx {
                    Mode::Rx
                } else {
                    continue;
                }
            }
            ModeCommand::TonePress => {
                if mode == Mode::Rx {
                    Mode::Tx
                } else {
                    continue;
                }
            }
            ModeCommand::ToneRelease => {
                if mode == Mode::Tx {
                    Mode::Rx
                } else {
                    continue;
                }
            }
        };
        mode = new_mode;
        MODE.signal(mode);
    }
}
