use embassy_futures::select::{select3, Either3};

use crate::{
    app::events::{CURRENT_FREQUENCY, CURRENT_MODE, CURRENT_RF_GAIN_MODE},
    peripherals::modules::bpf::Bpf,
};
use common::error::error;

#[embassy_executor::task]
pub async fn bpf_task(mut bpf: Bpf) {
    loop {
        match select3(
            CURRENT_RF_GAIN_MODE.wait(),
            CURRENT_MODE.wait(),
            CURRENT_FREQUENCY.wait(),
        )
        .await
        {
            Either3::First(rf_gain_mode) => {
                if let Err(e) = bpf.set_rf_gain_mode(rf_gain_mode).await {
                    error(e).await;
                }
            }
            Either3::Second(mode) => {
                if let Err(e) = bpf.set_mode(mode).await {
                    error(e).await;
                }
            }
            Either3::Third(frequency) => {
                if let Err(e) = bpf.set_frequency(frequency).await {
                    error(e).await;
                }
            }
        }
    }
}
