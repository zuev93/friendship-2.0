use common::error::error;
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_stm32::peripherals as stm_peripherals;
use embassy_stm32::sai::Sai;
use static_cell::StaticCell;

use crate::{
    app::events::{AUDIO_BUFFER_HEADPHONES, CURRENT_MODE, CURRENT_VOLUME},
    consts::AUDIO_BUFFER_SIZE,
    front_panel::{events::AUDIO_MIC_BUFFER, modules::audio::Audio},
};

type SaiTx = Sai<'static, stm_peripherals::SAI2, u16>;
type SaiRx = Sai<'static, stm_peripherals::SAI2, u16>;

static AUDIO_PANEL: StaticCell<Audio> = StaticCell::new();

pub async fn create_tasks(spawner: Spawner, audio: Audio) {
    let audio_panel = AUDIO_PANEL.init(audio);
    let (sai_tx, sai_rx) = audio_panel.split_sai();
    spawner.must_spawn(audio_panel_sai_mic_task(sai_rx));
    spawner.must_spawn(audio_panel_sai_headphones_task(sai_tx));
    spawner.must_spawn(control_task(audio_panel));
}

#[embassy_executor::task]
async fn audio_panel_sai_mic_task(sai_rx: &'static mut SaiRx) {
    let mut buffer: [u16; AUDIO_BUFFER_SIZE] = [0u16; AUDIO_BUFFER_SIZE];

    loop {
        loop {
            let result = sai_rx.read(&mut buffer).await;
            if let Err(_) = result {
                error("Failed to read audio panel SAI stream").await;
                continue;
            }
            AUDIO_MIC_BUFFER.signal(buffer);
        }
    }
}

#[embassy_executor::task]
async fn audio_panel_sai_headphones_task(sai_tx: &'static mut SaiTx) {
    loop {
        let buffer = AUDIO_BUFFER_HEADPHONES.wait().await;
        let result = sai_tx.write(&buffer).await;
        if let Err(_) = result {
            error("Failed to write audio panel SAI stream").await;
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
                let percent = (volume.raw().max(0) as u32 * 100 / 1000) as u8;
                if let Err(_) = audio.set_volume(percent).await {
                    error("Failed to set audio volume").await;
                }
            }
        }
    }
}
