use embassy_futures::select::{select5, Either5};

use crate::{
    app::events::{CLARIFIER_MODE, CLARIFIER_VALUE, FILTER, FREQUENCY, MODE},
    main_board::modules::mixer::Mixer,
};
use common::error::error;

#[embassy_executor::task]
pub async fn mixer_tasks(mut mixer: Mixer) {
    loop {
        match select5(
            FREQUENCY.wait(),
            CLARIFIER_MODE.wait(),
            MODE.wait(),
            CLARIFIER_VALUE.wait(),
            FILTER.wait(),
        )
        .await
        {
            Either5::First(frequency) => {
                if let Err(e) = mixer.set_frequency(frequency).await {
                    error(e).await;
                }
            }
            Either5::Second(clarifier_mode) => {
                if let Err(e) = mixer.set_clarifier_mode(clarifier_mode).await {
                    error(e).await;
                }
            }
            Either5::Third(mode) => {
                if let Err(e) = mixer.set_mode(mode).await {
                    error(e).await;
                }
            }
            Either5::Fourth(clarifier_value) => {
                if let Err(e) = mixer.set_clarifier_value(clarifier_value).await {
                    error(e).await;
                }
            }
            Either5::Fifth(filter) => {
                if let Err(e) = mixer.set_filter(filter).await {
                    error(e).await;
                }
            }
        }
    }
}
