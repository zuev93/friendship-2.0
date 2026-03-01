use embassy_futures::select::{select3, Either3};

use crate::{
    app::events::{FREQUENCY, MODE, RF_GAIN_MODE},
    peripherals::modules::bpf::Bpf,
    runtime_stats::TaskId,
};
use common::error::error;
use druzhba_macros::instrumented;

#[instrumented(TaskId::BpfTask)]
#[embassy_executor::task]
pub async fn bpf_task(mut bpf: Bpf) {
    let mut rf_gain_mode_rcv = RF_GAIN_MODE.receiver().unwrap();
    let mut mode_rcv = MODE.receiver().unwrap();
    let mut frequency_rcv = FREQUENCY.receiver().unwrap();
    loop {
        match select3(
            rf_gain_mode_rcv.changed(),
            mode_rcv.changed(),
            frequency_rcv.changed(),
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
