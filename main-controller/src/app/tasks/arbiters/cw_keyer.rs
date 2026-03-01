use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::events::{CW_BREAK_IN_DELAY, CW_KEY_MODE, CW_PADDLE_SWAP, CW_WEIGHT, CW_WPM};
use crate::app::types::{CwBreakInDelay, CwKeyMode, CwWeight, CwWpm};

pub enum CwKeyerCommand {
    CycleMode,
    WpmDelta(i8),
    WeightDelta(i8),
    ToggleSwap,
    BreakInDelayDelta(i16),
}

pub static CW_KEYER_CMD: Signal<ThreadModeRawMutex, CwKeyerCommand> = Signal::new();

#[embassy_executor::task]
pub async fn cw_keyer_arbiter_task() {
    let mut mode = CwKeyMode::IambicB;
    let mut wpm = CwWpm::new(20);
    let mut weight = CwWeight::new(50);
    let mut swap = false;
    let mut break_in_delay = CwBreakInDelay::new(500);

    loop {
        let cmd = CW_KEYER_CMD.wait().await;
        match cmd {
            CwKeyerCommand::CycleMode => {
                mode = mode.next();
                CW_KEY_MODE.sender().send(mode);
            }
            CwKeyerCommand::WpmDelta(delta) => {
                let new_val = wpm.add(delta);
                if new_val != wpm {
                    wpm = new_val;
                    CW_WPM.sender().send(wpm);
                }
            }
            CwKeyerCommand::WeightDelta(delta) => {
                let new_val = weight.add(delta);
                if new_val != weight {
                    weight = new_val;
                    CW_WEIGHT.sender().send(weight);
                }
            }
            CwKeyerCommand::ToggleSwap => {
                swap = !swap;
                CW_PADDLE_SWAP.sender().send(swap);
            }
            CwKeyerCommand::BreakInDelayDelta(delta) => {
                let new_val = break_in_delay.add(delta);
                if new_val != break_in_delay {
                    break_in_delay = new_val;
                    CW_BREAK_IN_DELAY.sender().send(break_in_delay);
                }
            }
        }
    }
}
