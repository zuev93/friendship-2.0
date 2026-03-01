use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::events::{SQUELCH, SQUELCH_ENABLED};
use crate::app::types::Squelch;

pub enum SquelchCommand {
    Delta(i16),
    Toggle,
}

pub static SQUELCH_CMD: Signal<ThreadModeRawMutex, SquelchCommand> = Signal::new();

#[embassy_executor::task]
pub async fn squelch_arbiter_task() {
    let mut squelch = Squelch::new(0);
    let mut enabled = false;

    loop {
        let cmd = SQUELCH_CMD.wait().await;
        match cmd {
            SquelchCommand::Delta(delta) => {
                let new_val = Squelch::new(squelch.raw() + delta);
                if new_val != squelch {
                    squelch = new_val;
                    SQUELCH.signal(squelch);
                }
            }
            SquelchCommand::Toggle => {
                enabled = !enabled;
                SQUELCH_ENABLED.signal(enabled);
            }
        }
    }
}
