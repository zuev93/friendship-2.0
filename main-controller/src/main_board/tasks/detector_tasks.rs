use embassy_futures::select::{select3, Either3};

use crate::{
    app::events::{FILTER, MODE, TRANSMIT_MODE},
    main_board::modules::detector::Detector,
};
use common::error::error;

#[embassy_executor::task]
pub async fn detector_tasks(mut detector: Detector) {
    loop {
        match select3(
            TRANSMIT_MODE.wait(),
            MODE.wait(),
            FILTER.wait(),
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
