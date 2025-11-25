use embassy_futures::select::{select, Either};

use crate::{
    app::events::{CURRENT_MODE, CURRENT_RF_POWER},
    main_board::modules::power_control::TxPowerControl,
};
use common::error::error;

#[embassy_executor::task]
pub async fn power_control_task(mut power_control: TxPowerControl) {
    loop {
        match select(CURRENT_RF_POWER.wait(), CURRENT_MODE.wait()).await {
            Either::First(rf_power) => {
                if let Err(e) = power_control.set_power(rf_power).await {
                    error(e).await;
                }
            }
            Either::Second(mode) => {
                if let Err(e) = power_control.set_mode(mode).await {
                    error(e).await;
                }
            }
        }
    }
}
