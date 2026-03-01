use common::error::error;
use embassy_executor::Spawner;
use static_cell::StaticCell;

use crate::{
    app::events::{AUDIO_BUFFER_SPEAKERS, MODE},
    control_board::modules::audio::Audio,
    runtime_stats::TaskId,
};
use druzhba_macros::instrumented;

static AUDIO_PANEL: StaticCell<Audio> = StaticCell::new();

pub async fn create_tasks(spawner: Spawner, audio: Audio) {
    let audio_panel = AUDIO_PANEL.init(audio);
    spawner.must_spawn(audio_panel_i2s_speakers_task(audio_panel));
    spawner.must_spawn(control_task(audio_panel));
}

#[instrumented(TaskId::AudioI2sSpeakers)]
#[embassy_executor::task]
async fn audio_panel_i2s_speakers_task(audio: &'static Audio) {
    let mut spk_rcv = AUDIO_BUFFER_SPEAKERS.receiver().unwrap();
    loop {
        let buffer = spk_rcv.changed().await;
        if let Err(e) = audio.write(&buffer).await {
            error(e).await;
        }
    }
}

#[instrumented(TaskId::ControlBoardAudioControl)]
#[embassy_executor::task]
async fn control_task(audio: &'static Audio) {
    let mut mode_rcv = MODE.receiver().unwrap();
    loop {
        let mode = mode_rcv.changed().await;
        if let Err(e) = audio.set_mode(mode).await {
            error(e).await;
        }
    }
}
