use embassy_futures::select::{select4, Either4};

use crate::{
    app::events::{CURRENT_IF_GAIN, CURRENT_IF_GAIN_MODE, CURRENT_MODE},
    main_board::{events::CURRENT_RSSI, modules::if_gain_control::IfGainControl},
};
use common::error::error;

#[embassy_executor::task]
pub async fn if_gain_control_task(mut if_gain_control: IfGainControl) {
    loop {
        match select4(
            CURRENT_IF_GAIN.wait(),
            CURRENT_IF_GAIN_MODE.wait(),
            CURRENT_RSSI.wait(),
            CURRENT_MODE.wait(),
        )
        .await
        {
            Either4::First(if_gain) => {
                if let Err(e) = if_gain_control.set_manual_gain_raw(if_gain).await {
                    error(e).await;
                }
            }
            Either4::Second(if_gain_mode) => {
                if let Err(e) = if_gain_control.set_if_gain_mode(if_gain_mode).await {
                    error(e).await;
                }
            }
            Either4::Third(rssi) => {
                if let Err(e) = if_gain_control.update_agc(rssi).await {
                    error(e).await;
                }
            }
            Either4::Fourth(mode) => {
                if let Err(e) = if_gain_control.set_mode(mode).await {
                    error(e).await;
                }
            }
        }
    }
}
