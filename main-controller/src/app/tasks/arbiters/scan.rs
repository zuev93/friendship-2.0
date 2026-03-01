use druzhba_macros::instrumented;
use crate::runtime_stats::TaskId;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::events::{SCAN_ENABLED, SCAN_RESUME, SCAN_STEP};
use crate::app::types::{ScanResume, ScanStep};

pub enum ScanCommand {
    Toggle,
    Stop,
    StepDelta(i16),
    ResumeDelta(i16),
}

pub static SCAN_CMD: Signal<ThreadModeRawMutex, ScanCommand> = Signal::new();

#[instrumented(TaskId::ScanArbiter)]
#[embassy_executor::task]
pub async fn scan_arbiter_task() {
    let mut enabled = false;
    let mut step = ScanStep::new(1000);
    let mut resume = ScanResume::new(3);

    loop {
        let cmd = SCAN_CMD.wait().await;
        match cmd {
            ScanCommand::Toggle => {
                enabled = !enabled;
                SCAN_ENABLED.sender().send(enabled);
            }
            ScanCommand::Stop => {
                if enabled {
                    enabled = false;
                    SCAN_ENABLED.sender().send(false);
                }
            }
            ScanCommand::StepDelta(delta) => {
                let new_val = step.add(delta);
                if new_val != step {
                    step = new_val;
                    SCAN_STEP.sender().send(step);
                }
            }
            ScanCommand::ResumeDelta(delta) => {
                let new_val = ScanResume::new((resume.raw() as i16 + delta) as u8);
                if new_val != resume {
                    resume = new_val;
                    SCAN_RESUME.sender().send(resume);
                }
            }
        }
    }
}
