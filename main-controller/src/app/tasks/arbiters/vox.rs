use druzhba_macros::instrumented;
use crate::runtime_stats::TaskId;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::events::{VOX_ANTI_TRIP, VOX_DELAY, VOX_ENABLED, VOX_GAIN};
use crate::app::types::{VoxAntiTrip, VoxDelay, VoxGain};

pub enum VoxCommand {
    Toggle,
    GainDelta(i16),
    DelayDelta(i16),
    AntiTripDelta(i16),
}

pub static VOX_CMD: Signal<ThreadModeRawMutex, VoxCommand> = Signal::new();

#[instrumented(TaskId::VoxArbiter)]
#[embassy_executor::task]
pub async fn vox_arbiter_task() {
    let mut enabled = false;
    let mut gain = VoxGain::new(500);
    let mut delay = VoxDelay::new(500);
    let mut anti_trip = VoxAntiTrip::new(500);

    loop {
        let cmd = VOX_CMD.wait().await;
        match cmd {
            VoxCommand::Toggle => {
                enabled = !enabled;
                VOX_ENABLED.sender().send(enabled);
            }
            VoxCommand::GainDelta(delta) => {
                let new_val = VoxGain::new(gain.raw() + delta);
                if new_val != gain {
                    gain = new_val;
                    VOX_GAIN.sender().send(gain);
                }
            }
            VoxCommand::DelayDelta(delta) => {
                let new_val = VoxDelay::new(delay.raw() + delta);
                if new_val != delay {
                    delay = new_val;
                    VOX_DELAY.sender().send(delay);
                }
            }
            VoxCommand::AntiTripDelta(delta) => {
                let new_val = VoxAntiTrip::new(anti_trip.raw() + delta);
                if new_val != anti_trip {
                    anti_trip = new_val;
                    VOX_ANTI_TRIP.sender().send(anti_trip);
                }
            }
        }
    }
}
