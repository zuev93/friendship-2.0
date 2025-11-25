use embassy_futures::select::{select5, Either5};

use crate::{
    app::events::{
        CURRENT_CLARIFIER_MODE, CURRENT_CLARIFIER_VALUE, CURRENT_FILTER, CURRENT_FREQUENCY,
        CURRENT_MODE,
    },
    main_board::modules::dds::DDS,
};
use common::error::error;

#[embassy_executor::task]
pub async fn dds_control_task(mut dds: DDS) {
    loop {
        match select5(
            CURRENT_FREQUENCY.wait(),
            CURRENT_CLARIFIER_MODE.wait(),
            CURRENT_MODE.wait(),
            CURRENT_CLARIFIER_VALUE.wait(),
            CURRENT_FILTER.wait(),
        )
        .await
        {
            Either5::First(frequency) => {
                if let Err(e) = dds.set_frequency(frequency).await {
                    error(e).await;
                }
            }
            Either5::Second(clarifier_mode) => {
                if let Err(e) = dds.set_clarifier_mode(clarifier_mode).await {
                    error(e).await;
                }
            }
            Either5::Third(mode) => {
                if let Err(e) = dds.set_mode(mode).await {
                    error(e).await;
                }
            }
            Either5::Fourth(clarifier_value) => {
                if let Err(e) = dds.set_clarifier_value(clarifier_value).await {
                    error(e).await;
                }
            }
            Either5::Fifth(filter) => {
                if let Err(e) = dds.set_filter(filter).await {
                    error(e).await;
                }
            }
        }
    }
}
