use embassy_futures::select::{select3, Either3};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::{
    events::{BAND, FREQUENCY, RSSI_FAST_MODE, SWEEP_REQUEST},
    types::{Band, Frequency, SweepRequest},
};

pub enum BandCommand {
    Up,
    Down,
}

pub static FREQUENCY_CMD: Signal<ThreadModeRawMutex, i32> = Signal::new();
pub static BAND_CMD: Signal<ThreadModeRawMutex, BandCommand> = Signal::new();

#[embassy_executor::task]
pub async fn frequency_arbiter_task() {
    let mut current_band = Band::Band20m;
    let mut current_frequency: Frequency = 14_200_000;
    let mut user_freq = current_frequency;
    let mut sweep_active = false;

    let mut last_signaled_band = current_band;
    let mut sweep_request_rcv = SWEEP_REQUEST.receiver().unwrap();

    loop {
        match select3(
            FREQUENCY_CMD.wait(),
            BAND_CMD.wait(),
            sweep_request_rcv.changed(),
        )
        .await
        {
            Either3::First(delta) => {
                current_frequency = current_frequency.saturating_add_signed(delta);

                if current_frequency > current_band.upper_frequency() {
                    current_band = current_band.next();
                    current_frequency = current_band.lower_frequency();
                } else if current_frequency < current_band.lower_frequency() {
                    current_band = current_band.prev();
                    current_frequency = current_band.upper_frequency();
                }

                if current_band != last_signaled_band {
                    BAND.sender().send(current_band);
                    last_signaled_band = current_band;
                }

                user_freq = current_frequency;
                FREQUENCY.sender().send(current_frequency);
            }
            Either3::Second(cmd) => {
                match cmd {
                    BandCommand::Up => {
                        current_band = current_band.next();
                        current_frequency = current_band.lower_frequency();
                    }
                    BandCommand::Down => {
                        current_band = current_band.prev();
                        current_frequency = current_band.upper_frequency();
                    }
                }

                if current_band != last_signaled_band {
                    BAND.sender().send(current_band);
                    last_signaled_band = current_band;
                }

                user_freq = current_frequency;
                FREQUENCY.sender().send(current_frequency);
            }
            Either3::Third(req) => match req {
                SweepRequest::SetFrequency(freq) => {
                    if !sweep_active {
                        sweep_active = true;
                        RSSI_FAST_MODE.sender().send(true);
                    }
                    FREQUENCY.sender().send(freq);
                }
                SweepRequest::Done => {
                    sweep_active = false;
                    RSSI_FAST_MODE.sender().send(false);
                    FREQUENCY.sender().send(user_freq);
                }
            },
        }
    }
}
