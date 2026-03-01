use druzhba_macros::instrumented;
use crate::runtime_stats::TaskId;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::events::RF_GAIN_MODE;
use crate::app::types::RfGainMode;

pub enum RfGainModeCommand {
    Cycle,
    Set(RfGainMode),
}

pub static RF_GAIN_MODE_CMD: Signal<ThreadModeRawMutex, RfGainModeCommand> = Signal::new();

#[instrumented(TaskId::RfGainModeArbiter)]
#[embassy_executor::task]
pub async fn rf_gain_mode_arbiter_task() {
    let mut rf_gain_mode = RfGainMode::Normal;

    loop {
        let cmd = RF_GAIN_MODE_CMD.wait().await;
        match cmd {
            RfGainModeCommand::Cycle => {
                rf_gain_mode = rf_gain_mode.next();
                RF_GAIN_MODE.sender().send(rf_gain_mode);
            }
            RfGainModeCommand::Set(mode) => {
                rf_gain_mode = mode;
                RF_GAIN_MODE.sender().send(rf_gain_mode);
            }
        }
    }
}
