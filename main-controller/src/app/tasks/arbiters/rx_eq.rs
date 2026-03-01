use druzhba_macros::instrumented;
use crate::runtime_stats::TaskId;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::events::{RX_EQ_ENABLED, RX_EQ_HIGH, RX_EQ_LOW, RX_EQ_MID};
use crate::app::types::EqGain;

pub enum RxEqCommand {
    Toggle,
    SetLow(i8),
    SetMid(i8),
    SetHigh(i8),
}

pub static RX_EQ_CMD: Signal<ThreadModeRawMutex, RxEqCommand> = Signal::new();

#[instrumented(TaskId::RxEqArbiter)]
#[embassy_executor::task]
pub async fn rx_eq_arbiter_task() {
    let mut enabled = false;
    let mut low = EqGain::new(0);
    let mut mid = EqGain::new(0);
    let mut high = EqGain::new(0);

    loop {
        let cmd = RX_EQ_CMD.wait().await;
        match cmd {
            RxEqCommand::Toggle => {
                enabled = !enabled;
                RX_EQ_ENABLED.sender().send(enabled);
            }
            RxEqCommand::SetLow(delta) => {
                let new_val = low.add(delta);
                if new_val != low {
                    low = new_val;
                    RX_EQ_LOW.sender().send(low);
                }
            }
            RxEqCommand::SetMid(delta) => {
                let new_val = mid.add(delta);
                if new_val != mid {
                    mid = new_val;
                    RX_EQ_MID.sender().send(mid);
                }
            }
            RxEqCommand::SetHigh(delta) => {
                let new_val = high.add(delta);
                if new_val != high {
                    high = new_val;
                    RX_EQ_HIGH.sender().send(high);
                }
            }
        }
    }
}
