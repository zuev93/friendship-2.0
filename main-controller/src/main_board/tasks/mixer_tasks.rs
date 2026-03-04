use embassy_futures::select::{select, select3, Either, Either3};

use crate::{
    app::events::{CLARIFIER_MODE, CLARIFIER_VALUE, FILTER, FREQUENCY, MODE, TRANSMIT_MODE},
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
    let mut transmit_mode_rcv = TRANSMIT_MODE.receiver().unwrap();
    loop {
        match select(
            select3(
                frequency_rcv.changed(),
                clarifier_mode_rcv.changed(),
                mode_rcv.changed(),
            ),
            select3(
                clarifier_value_rcv.changed(),
                filter_rcv.changed(),
                transmit_mode_rcv.changed(),
            ),
        )
        .await
        {
            Either::First(inner) => match inner {
                Either3::First(frequency) => {
                    if let Err(e) = mixer.set_frequency(frequency).await {
                        error(e).await;
                    }
                }
                Either3::Second(clarifier_mode) => {
                    if let Err(e) = mixer.set_clarifier_mode(clarifier_mode).await {
                        error(e).await;
                    }
                }
                Either3::Third(mode) => {
                    if let Err(e) = mixer.set_mode(mode).await {
                        error(e).await;
                    }
                }
            },
            Either::Second(inner) => match inner {
                Either3::First(clarifier_value) => {
                    if let Err(e) = mixer.set_clarifier_value(clarifier_value).await {
                        error(e).await;
                    }
                }
                Either3::Second(filter) => {
                    if let Err(e) = mixer.set_filter(filter).await {
                        error(e).await;
                    }
                }
                Either3::Third(transmit_mode) => {
                    if let Err(e) = mixer.set_transmit_mode(transmit_mode).await {
                        error(e).await;
                    }
                }
            },
        }
    }
}
