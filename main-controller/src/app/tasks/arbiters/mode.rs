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
    VoxActivate,
    VoxDeactivate,
}

pub static MODE_CMD: Signal<ThreadModeRawMutex, ModeCommand> = Signal::new();

#[embassy_executor::task]
pub async fn mode_arbiter_task() {
    let mut mode = Mode::StandBy;
    let mut ptt_held = false;
    let mut vox_active = false;

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
                ptt_held = true;
                if mode == Mode::Rx {
                    Mode::Tx
                } else {
                    continue;
                }
            }
            ModeCommand::TransmitRelease => {
                ptt_held = false;
                if mode == Mode::Tx && !vox_active {
                    Mode::Rx
                } else {
                    continue;
                }
            }
            ModeCommand::TonePress => {
                ptt_held = true;
                if mode == Mode::Rx {
                    Mode::Tx
                } else {
                    continue;
                }
            }
            ModeCommand::ToneRelease => {
                ptt_held = false;
                if mode == Mode::Tx && !vox_active {
                    Mode::Rx
                } else {
                    continue;
                }
            }
            ModeCommand::VoxActivate => {
                vox_active = true;
                if mode == Mode::Rx {
                    Mode::Tx
                } else {
                    continue;
                }
            }
            ModeCommand::VoxDeactivate => {
                vox_active = false;
                if mode == Mode::Tx && !ptt_held {
                    Mode::Rx
                } else {
                    continue;
                }
            }
        };
        mode = new_mode;
        MODE.sender().send(mode);
    }
}
