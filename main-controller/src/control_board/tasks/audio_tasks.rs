use common::error::error;
use embassy_executor::Spawner;
use embassy_stm32::i2s::Writer;
use static_cell::StaticCell;

use crate::{
    app::events::{AUDIO_BUFFER_SPEAKERS, CURRENT_MODE},
    control_board::modules::audio::Audio,
};

type AudioWriter = Writer<'static, 'static, u16>;

static AUDIO_PANEL: StaticCell<Audio> = StaticCell::new();
static AUDIO_I2S_WRITER: StaticCell<AudioWriter> = StaticCell::new();

pub async fn create_tasks(spawner: Spawner, audio: Audio) {
    let audio_panel = AUDIO_PANEL.init(audio);
    let writer = audio_panel.get_writer().await;
    let writer = AUDIO_I2S_WRITER.init(writer);
    spawner.must_spawn(audio_panel_i2s_speakers_task(writer));
    spawner.must_spawn(control_task(audio_panel));
}

#[embassy_executor::task]
async fn audio_panel_i2s_speakers_task(writer: &'static mut AudioWriter) {
    loop {
        let buffer = AUDIO_BUFFER_SPEAKERS.wait().await;
        let result = writer.write(&buffer).await;
        if let Err(_) = result {
            error("Failed to write audio panel I2S stream").await;
            continue;
        }
    }
}

#[embassy_executor::task]
async fn control_task(audio: &'static mut Audio) {
    loop {
        let mode = CURRENT_MODE.wait().await;
        if let Err(e) = audio.set_mode(mode).await {
            error(e).await;
        }
    }
}
