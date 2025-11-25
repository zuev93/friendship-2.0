use common::error::error;
use embassy_executor::Spawner;
use embassy_stm32::i2s::{self, Reader, Writer};

use crate::{
    app::{
        events::{AUDIO_BUFFER_TX, CURRENT_MODE},
        types::Mode,
    },
    consts::AUDIO_BUFFER_SIZE,
    main_board::{events::AUDIO_RX_BUFFER, modules::audio_panel::AudioPanel},
};

pub fn create_tasks(spawner: Spawner, mut audio_panel: AudioPanel) {
    let (reader, _) = audio_panel.split_i2s();
    spawner.must_spawn(audio_panel_i2s_rx_task(Rdr::new(reader)));
    // spawner.must_spawn(audio_panel_i2s_tx_task(writer));
    // spawner.must_spawn(control_task(audio_panel));
}

#[embassy_executor::task]
pub async fn audio_panel_i2s_rx_task(reader: Rdr) {
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
pub async fn audio_panel_i2s_tx_task(mut writer: Writer<'static, 'static, u16>) {
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
pub async fn control_task(audio_panel: &'static mut AudioPanel) {
    loop {
        // TODO process other settings such as volume
        let mode = CURRENT_MODE.wait().await;

        let result = match mode {
            Mode::Rx => audio_panel.set_signal_detector_to_adc().await,
            Mode::Tx => audio_panel.set_signal_detector_to_dac().await,
            Mode::StandBy | Mode::WarmUp => audio_panel.set_signal_detector_to_adc().await,
        };

        if let Err(e) = result {
            error(e).await;
        }
    }
}
