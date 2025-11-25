use common::error::error;

use crate::{app::events::CURRENT_VOLUME, front_panel::modules::audio::Audio};

#[embassy_executor::task]
pub async fn audio_task(audio: Audio) {
    loop {
        let volume = CURRENT_VOLUME.wait().await;

        if let Err(_) = audio.set_volume(volume as u8).await {
            error("Failed to set audio volume").await;
        }
    }
}
