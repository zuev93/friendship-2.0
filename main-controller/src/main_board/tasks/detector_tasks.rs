use embassy_futures::select::{select3, Either3};

use crate::{
    app::events::{FILTER, MODE, TRANSMIT_MODE},
    main_board::modules::detector::Detector,
    runtime_stats::TaskId,
};
use common::error::error;
use druzhba_macros::instrumented;

#[instrumented(TaskId::DetectorTasks)]
#[embassy_executor::task]
pub async fn detector_tasks(mut detector: Detector) {
    let mut transmit_mode_rcv = TRANSMIT_MODE.receiver().unwrap();
    let mut mode_rcv = MODE.receiver().unwrap();
    let mut filter_rcv = FILTER.receiver().unwrap();
    loop {
        match select3(
            transmit_mode_rcv.changed(),
            mode_rcv.changed(),
            filter_rcv.changed(),
        )
        .await
        {
            Either3::First(transmit_mode) => {
                if let Err(e) = detector.set_transmit_mode(transmit_mode).await {
                    error(e).await;
                }
            }
            Either3::Second(mode) => {
                if let Err(e) = detector.set_mode(mode).await {
                    error(e).await;
                }
            }
            Either3::Third(filter) => {
                if let Err(e) = detector.set_filter(filter).await {
                    error(e).await;
                }
            }
        }
    }
}
