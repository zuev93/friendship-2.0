use crate::{app::events::CURRENT_MODE, peripherals::modules::hf_amp::HfAmp};
use common::error::error;

#[embassy_executor::task]
pub async fn hf_amp_task(mut hf_amp: HfAmp) {
    loop {
        let mode = CURRENT_MODE.wait().await;
        if let Err(e) = hf_amp.set_mode(mode).await {
            error(e).await;
        }
    }
}
