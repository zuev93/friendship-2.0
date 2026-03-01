use common::error::error;
use embassy_executor::Spawner;
use static_cell::StaticCell;

use crate::{
    app::events::{AUDIO_BUFFER_SPEAKERS, MODE},
    control_board::modules::audio::Audio,
};

static AUDIO_PANEL: StaticCell<Audio> = StaticCell::new();

pub async fn create_tasks(spawner: Spawner, audio: Audio) {
    let audio_panel = AUDIO_PANEL.init(audio);
    spawner.must_spawn(audio_panel_i2s_speakers_task(audio_panel));
    spawner.must_spawn(control_task(audio_panel));
}

#[embassy_executor::task]
async fn audio_panel_i2s_speakers_task(audio: &'static Audio) {
    loop {
        let buffer = AUDIO_BUFFER_SPEAKERS.wait().await;
        if let Err(e) = audio.write(&buffer).await {
            error(e).await;
        }
    }
}

#[embassy_executor::task]
async fn control_task(audio: &'static Audio) {
    loop {
        let mode = MODE.wait().await;
        if let Err(e) = audio.set_mode(mode).await {
            error(e).await;
        }
    }
}
