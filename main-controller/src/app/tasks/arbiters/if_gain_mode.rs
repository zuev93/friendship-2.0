use druzhba_macros::instrumented;
use crate::runtime_stats::TaskId;
use embassy_futures::select::{select, Either};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::events::{IF_GAIN_MODE, SWEEP_ACTIVE};
use crate::app::types::IfGainMode;

pub enum IfGainModeCommand {
    Toggle,
    Set(IfGainMode),
}

pub static IF_GAIN_MODE_CMD: Signal<ThreadModeRawMutex, IfGainModeCommand> = Signal::new();

#[instrumented(TaskId::IfGainModeArbiter)]
#[embassy_executor::task]
pub async fn if_gain_mode_arbiter_task() {
    let mut if_gain_mode = IfGainMode::Manual;
    let mut user_mode = if_gain_mode;
    let mut sweep_active_rcv = SWEEP_ACTIVE.receiver().unwrap();

    loop {
        match select(IF_GAIN_MODE_CMD.wait(), sweep_active_rcv.changed()).await {
            Either::First(cmd) => match cmd {
                IfGainModeCommand::Toggle => {
                    if_gain_mode = if_gain_mode.toggle();
                    user_mode = if_gain_mode;
                    IF_GAIN_MODE.sender().send(if_gain_mode);
                }
                IfGainModeCommand::Set(mode) => {
                    if_gain_mode = mode;
                    user_mode = if_gain_mode;
                    IF_GAIN_MODE.sender().send(if_gain_mode);
                }
            },
            Either::Second(active) => {
                if active {
                    user_mode = if_gain_mode;
                    if_gain_mode = IfGainMode::Manual;
                    IF_GAIN_MODE.sender().send(if_gain_mode);
                } else {
                    if_gain_mode = user_mode;
                    IF_GAIN_MODE.sender().send(if_gain_mode);
                }
            }
        }
    }
}
