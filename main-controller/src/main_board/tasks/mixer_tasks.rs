use embassy_futures::select::{select5, Either5};

use crate::{
    app::events::{CLARIFIER_MODE, CLARIFIER_VALUE, FILTER, FREQUENCY, MODE},
    main_board::modules::mixer::Mixer,
    runtime_stats::TaskId,
};
use common::error::error;
use druzhba_macros::instrumented;

#[instrumented(TaskId::MixerTasks)]
#[embassy_executor::task]
pub async fn mixer_tasks(mut mixer: Mixer) {
    let mut frequency_rcv = FREQUENCY.receiver().unwrap();
    let mut clarifier_mode_rcv = CLARIFIER_MODE.receiver().unwrap();
    let mut mode_rcv = MODE.receiver().unwrap();
    let mut clarifier_value_rcv = CLARIFIER_VALUE.receiver().unwrap();
    let mut filter_rcv = FILTER.receiver().unwrap();
    loop {
        match select5(
            frequency_rcv.changed(),
            clarifier_mode_rcv.changed(),
            mode_rcv.changed(),
            clarifier_value_rcv.changed(),
            filter_rcv.changed(),
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
