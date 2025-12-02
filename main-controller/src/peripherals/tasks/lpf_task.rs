use embassy_futures::select::{select3, Either3};
use embassy_time::{Duration, Timer};

use crate::{
    app::events::{COUPLER_METRICS, CURRENT_FREQUENCY, CURRENT_MODE},
    peripherals::modules::lpf::Lpf,
};
use common::error::error;

const COUPLER_SAMPLE_PERIOD: Duration = Duration::from_millis(200);

#[embassy_executor::task]
pub async fn lpf_task(mut lpf: Lpf) {
    loop {
        match select3(
            CURRENT_FREQUENCY.wait(),
            CURRENT_MODE.wait(),
            Timer::after(COUPLER_SAMPLE_PERIOD),
        )
        .await
        {
            Either3::First(frequency) => {
                if let Err(e) = lpf.set_frequency(frequency).await {
                    error(e).await;
                }
            }
            Either3::Second(mode) => {
                if let Err(e) = lpf.set_mode(mode).await {
                    error(e).await;
                }
            }
            Either3::Third(_) => {
                match lpf.read_coupler_metrics().await {
                    Ok(metrics) => COUPLER_METRICS.signal(metrics),
                    Err(e) => error(e).await,
                }
            }
        }
    }
}
