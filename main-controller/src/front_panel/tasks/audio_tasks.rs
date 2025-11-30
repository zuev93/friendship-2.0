use common::error::error;
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_stm32::i2s::{Reader, Writer};
use static_cell::StaticCell;

use crate::{
    app::events::{AUDIO_BUFFER_HEADPHONES, CURRENT_MODE, CURRENT_VOLUME},
    consts::AUDIO_BUFFER_SIZE,
    front_panel::{events::AUDIO_MIC_BUFFER, modules::audio::Audio},
};

type AudioReader = Reader<'static, 'static, u16>;
type AudioWriter = Writer<'static, 'static, u16>;

static AUDIO_PANEL: StaticCell<Audio> = StaticCell::new();
static AUDIO_I2S_READER: StaticCell<AudioReader> = StaticCell::new();
static AUDIO_I2S_WRITER: StaticCell<AudioWriter> = StaticCell::new();

pub async fn create_tasks(spawner: Spawner, audio: Audio) {
    let audio_panel = AUDIO_PANEL.init(audio);
    let (reader, writer) = audio_panel.split_i2s().await;
    let reader = AUDIO_I2S_READER.init(reader);
    let writer = AUDIO_I2S_WRITER.init(writer);
    spawner.must_spawn(audio_panel_i2s_mic_task(reader));
    spawner.must_spawn(audio_panel_i2s_headphones_task(writer));
    spawner.must_spawn(control_task(audio_panel));
}

#[embassy_executor::task]
async fn audio_panel_i2s_mic_task(reader: &'static mut AudioReader) {
    let mut buffer: [u16; AUDIO_BUFFER_SIZE] = [0u16; AUDIO_BUFFER_SIZE];

    loop {
        loop {
            let result = reader.read(&mut buffer).await;
            if let Err(_) = result {
                error("Failed to read audio panel I2S stream").await;
                continue;
            }
            AUDIO_MIC_BUFFER.signal(buffer);
        }
    }
}

#[embassy_executor::task]
async fn audio_panel_i2s_headphones_task(writer: &'static mut AudioWriter) {
    loop {
        let buffer = AUDIO_BUFFER_HEADPHONES.wait().await;
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
        match select(CURRENT_MODE.wait(), CURRENT_VOLUME.wait()).await {
            Either::First(mode) => {
                if let Err(e) = audio.set_mode(mode).await {
                    error(e).await;
                }
            }
            Either::Second(volume) => {
                if let Err(_) = audio.set_volume(volume as u8).await {
                    error("Failed to set audio volume").await;
                }
            }
        }
    }
}
