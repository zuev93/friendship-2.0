use embassy_futures::select::{select3, Either3};

use crate::app::{
    events::{
        CURRENT_FREQUENCY, CURRENT_IF_GAIN_MODE, RSSI_FAST_MODE, SWEEP_REQUEST, TARGET_FREQUENCY,
        TARGET_IF_GAIN_MODE,
    },
    types::{IfGainMode, SweepRequest},
};

#[embassy_executor::task]
pub async fn frequency_arbiter_task() {
    let mut stored_user_freq = 0;
    let mut stored_user_if_gain_mode = IfGainMode::Manual;
    let mut sweep_active = false;

    loop {
        match select3(
            CURRENT_FREQUENCY.wait(),
            CURRENT_IF_GAIN_MODE.wait(),
            SWEEP_REQUEST.wait(),
        )
        .await
        {
            Either3::First(user_freq) => {
                stored_user_freq = user_freq;
                if !sweep_active {
                    TARGET_FREQUENCY.signal(user_freq);
                }
            }
            Either3::Second(user_mode) => {
                stored_user_if_gain_mode = user_mode;
                if !sweep_active {
                    TARGET_IF_GAIN_MODE.signal(user_mode);
                }
            }
            Either3::Third(req) => match req {
                SweepRequest::SetFrequency(freq) => {
                    if !sweep_active {
                        sweep_active = true;
                        RSSI_FAST_MODE.signal(true);
                        TARGET_IF_GAIN_MODE.signal(IfGainMode::Manual);
                    }
                    TARGET_FREQUENCY.signal(freq);
                }
                SweepRequest::Done => {
                    sweep_active = false;
                    RSSI_FAST_MODE.signal(false);
                    TARGET_FREQUENCY.signal(stored_user_freq);
                    TARGET_IF_GAIN_MODE.signal(stored_user_if_gain_mode);
                }
            },
        }
    }
}
