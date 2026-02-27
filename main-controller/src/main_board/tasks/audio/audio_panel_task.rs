use common::error::error;
use embassy_executor::Spawner;
use embassy_stm32::i2s::{Reader, Writer};
use static_cell::StaticCell;

use crate::{
    app::events::{AUDIO_BUFFER_TX, CURRENT_MODE},
    consts::AUDIO_BUFFER_SIZE,
    main_board::{events::AUDIO_RX_BUFFER, modules::audio_panel::AudioPanel},
};

type AudioReader = Reader<'static, 'static, u16>;
type AudioWriter = Writer<'static, 'static, u16>;

static AUDIO_PANEL: StaticCell<AudioPanel> = StaticCell::new();
static AUDIO_I2S_READER: StaticCell<AudioReader> = StaticCell::new();
static AUDIO_I2S_WRITER: StaticCell<AudioWriter> = StaticCell::new();

pub async fn create_tasks(spawner: Spawner, audio_panel: AudioPanel) {
    let audio_panel = AUDIO_PANEL.init(audio_panel);
    let (reader, writer) = audio_panel.split_i2s();
    let reader = AUDIO_I2S_READER.init(reader);
    let writer = AUDIO_I2S_WRITER.init(writer);
    spawner.must_spawn(audio_panel_i2s_rx_task(reader));
    spawner.must_spawn(audio_panel_i2s_tx_task(writer));
    spawner.must_spawn(control_task(audio_panel));
}

#[embassy_executor::task]
async fn audio_panel_i2s_rx_task(reader: &'static mut AudioReader) {
    let mut buffer: [u16; AUDIO_BUFFER_SIZE] = [0u16; AUDIO_BUFFER_SIZE];

    loop {
        loop {
            let result = reader.read(&mut buffer).await;
            if let Err(_) = result {
                error("Failed to read audio panel I2S stream").await;
                continue;
            }
            AUDIO_RX_BUFFER.signal(buffer);
        }
    }
}

#[embassy_executor::task]
async fn audio_panel_i2s_tx_task(writer: &'static mut AudioWriter) {
    loop {
        let buffer = AUDIO_BUFFER_TX.wait().await;
        let result = writer.write(&buffer).await;
        if let Err(_) = result {
            error("Failed to write audio panel I2S stream").await;
            continue;
        }
    }
}

#[embassy_executor::task]
async fn control_task(audio_panel: &'static mut AudioPanel) {
    loop {
        let mode = CURRENT_MODE.wait().await;

        if let Err(e) = audio_panel.set_mode(mode).await {
            error(e).await;
        }
    }
}
