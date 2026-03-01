use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::events::{TX_EQ_ENABLED, TX_EQ_HIGH, TX_EQ_LOW, TX_EQ_MID};
use crate::app::types::EqGain;

pub enum TxEqCommand {
    Toggle,
    SetLow(i8),
    SetMid(i8),
    SetHigh(i8),
}

pub static TX_EQ_CMD: Signal<ThreadModeRawMutex, TxEqCommand> = Signal::new();

#[embassy_executor::task]
pub async fn tx_eq_arbiter_task() {
    let mut enabled = false;
    let mut low = EqGain::new(0);
    let mut mid = EqGain::new(0);
    let mut high = EqGain::new(0);

    loop {
        let cmd = TX_EQ_CMD.wait().await;
        match cmd {
            TxEqCommand::Toggle => {
                enabled = !enabled;
                TX_EQ_ENABLED.sender().send(enabled);
            }
            TxEqCommand::SetLow(delta) => {
                let new_val = low.add(delta);
                if new_val != low {
                    low = new_val;
                    TX_EQ_LOW.sender().send(low);
                }
            }
            TxEqCommand::SetMid(delta) => {
                let new_val = mid.add(delta);
                if new_val != mid {
                    mid = new_val;
                    TX_EQ_MID.sender().send(mid);
                }
            }
            TxEqCommand::SetHigh(delta) => {
                let new_val = high.add(delta);
                if new_val != high {
                    high = new_val;
                    TX_EQ_HIGH.sender().send(high);
                }
            }
        }
    }
}
