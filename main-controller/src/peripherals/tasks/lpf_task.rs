use embassy_futures::select::{select, Either};

use crate::{
    app::events::{CURRENT_FREQUENCY, CURRENT_MODE},
    peripherals::modules::lpf::Lpf,
};
use common::error::error;

#[embassy_executor::task]
pub async fn lpf_task(mut lpf: Lpf) {
    loop {
        match select(CURRENT_FREQUENCY.wait(), CURRENT_MODE.wait()).await {
            Either::First(frequency) => {
                if let Err(e) = lpf.set_frequency(frequency).await {
                    error(e).await;
                }
            }
            Either::Second(mode) => {
                if let Err(e) = lpf.set_mode(mode).await {
                    error(e).await;
                }
            }
        }
    }
}
