use druzhba_macros::instrumented;
use crate::runtime_stats::TaskId;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::events::MODE;
use crate::app::types::Mode;

pub enum ModeCommand {
    PowerToggle,
    PttFrontPress,
    PttFrontRelease,
    PttRearPress,
    PttRearRelease,
    PttCatPress,
    PttCatRelease,
    PttMicPress,
    PttMicRelease,
    TonePress,
    ToneRelease,
    VoxActivate,
    VoxDeactivate,
    CwKeyDown,
    CwKeyUp,
}

pub static MODE_CMD: Signal<ThreadModeRawMutex, ModeCommand> = Signal::new();

#[instrumented(TaskId::ModeArbiter)]
#[embassy_executor::task]
pub async fn mode_arbiter_task() {
    let mut mode = Mode::StandBy;
    let mut ptt_front = false;
    let mut ptt_rear = false;
    let mut ptt_cat = false;
    let mut ptt_mic = false;
    let mut tone_held = false;
    let mut vox_active = false;
    let mut cw_key_active = false;

    loop {
        let cmd = MODE_CMD.wait().await;

        match cmd {
            ModeCommand::PttFrontPress => ptt_front = true,
            ModeCommand::PttFrontRelease => ptt_front = false,
            ModeCommand::PttRearPress => ptt_rear = true,
            ModeCommand::PttRearRelease => ptt_rear = false,
            ModeCommand::PttCatPress => ptt_cat = true,
            ModeCommand::PttCatRelease => ptt_cat = false,
            ModeCommand::PttMicPress => ptt_mic = true,
            ModeCommand::PttMicRelease => ptt_mic = false,
            ModeCommand::TonePress => tone_held = true,
            ModeCommand::ToneRelease => tone_held = false,
            ModeCommand::VoxActivate => vox_active = true,
            ModeCommand::VoxDeactivate => vox_active = false,
            ModeCommand::CwKeyDown => cw_key_active = true,
            ModeCommand::CwKeyUp => cw_key_active = false,
            ModeCommand::PowerToggle => {}
        }

        let any_tx = ptt_front || ptt_rear || ptt_cat || ptt_mic
            || tone_held || vox_active || cw_key_active;

        let new_mode = match cmd {
            ModeCommand::PowerToggle => {
                if mode == Mode::StandBy { Mode::WarmUp } else { Mode::StandBy }
            }
            ModeCommand::PttFrontPress
            | ModeCommand::PttRearPress
            | ModeCommand::PttCatPress
            | ModeCommand::PttMicPress
            | ModeCommand::TonePress
            | ModeCommand::VoxActivate
            | ModeCommand::CwKeyDown => {
                if mode == Mode::Rx { Mode::Tx } else { continue; }
            }
            ModeCommand::PttFrontRelease
            | ModeCommand::PttRearRelease
            | ModeCommand::PttCatRelease
            | ModeCommand::PttMicRelease
            | ModeCommand::ToneRelease
            | ModeCommand::VoxDeactivate
            | ModeCommand::CwKeyUp => {
                if mode == Mode::Tx && !any_tx { Mode::Rx } else { continue; }
            }
        };
        mode = new_mode;
        MODE.sender().send(mode);
    }
}
