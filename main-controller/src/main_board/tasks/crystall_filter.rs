use embassy_futures::select::{select4, Either4};

use crate::{
    app::events::{CURRENT_FILTER, CURRENT_MODE, CURRENT_RF_GAIN_MODE, CURRENT_RF_POWER},
    main_board::modules::crystall_filter::CrystallFilter,
};
use common::error::error;

#[embassy_executor::task]
pub async fn crystall_filter_task(mut crystall_filter: CrystallFilter) {
    loop {
        match select4(
            CURRENT_RF_POWER.wait(),
            CURRENT_MODE.wait(),
            CURRENT_FILTER.wait(),
            CURRENT_RF_GAIN_MODE.wait(),
        )
        .await
        {
            Either4::First(rf_power) => {
                if let Err(e) = crystall_filter.set_power(rf_power).await {
                    error(e).await;
                }
            }
            Either4::Second(mode) => {
                if let Err(e) = crystall_filter.set_mode(mode).await {
                    error(e).await;
                }
            }
            Either4::Third(filter) => {
                if let Err(e) = crystall_filter.set_filter_type(filter).await {
                    error(e).await;
                }
            }
            Either4::Fourth(rf_gain_mode) => {
                if let Err(e) = crystall_filter.set_rf_gain_mode(rf_gain_mode).await {
                    error(e).await;
                }
            }
        }
    }
}
