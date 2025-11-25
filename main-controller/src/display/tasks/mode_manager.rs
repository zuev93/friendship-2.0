use crate::{
    app::{events::CURRENT_MODE, types::Mode},
    display::types::DisplayHardwareMutex,
};
use common::error::error;


#[embassy_executor::task]
pub async fn display_mode_manager(hardware: DisplayHardwareMutex) {
    loop {
        let new_mode = CURRENT_MODE.wait().await;

        match new_mode {
            Mode::StandBy => {
                // TODO add turn off
            }
            Mode::WarmUp => {
                let mut hw_guard = hardware.lock().await;
                if let Err(e) = hw_guard.display1.init().await {
                    error(e).await;
                }
                if let Err(e) = hw_guard.display2.init().await {
                    error(e).await;
                }
            }

            Mode::Rx => {}

            Mode::Tx => {}
        }
    }
}
