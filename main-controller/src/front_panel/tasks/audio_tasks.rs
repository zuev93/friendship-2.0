use common::error::error;
use embassy_executor::Spawner;
use embassy_futures::select::{select3, Either3};
use embassy_stm32::peripherals as stm_peripherals;
use embassy_stm32::sai::Sai;
use static_cell::StaticCell;

use crate::{
    app::events::{AUDIO_BUFFER_HEADPHONES, MICROPHONE, MODE, VOLUME},
    consts::AUDIO_BUFFER_SIZE,
    front_panel::{events::AUDIO_MIC_BUFFER, modules::audio::Audio},
    runtime_stats::TaskId,
};
use druzhba_macros::instrumented;

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
            AUDIO_MIC_BUFFER.sender().send(buffer);
        }
    }
}

#[instrumented(TaskId::FrontPanelAudioHeadphones)]
#[embassy_executor::task]
async fn audio_panel_sai_headphones_task(sai_tx: &'static mut SaiTx) {
    let mut hp_rcv = AUDIO_BUFFER_HEADPHONES.receiver().unwrap();
    loop {
        let buffer = hp_rcv.changed().await;
        let result = sai_tx.write(&buffer).await;
        if let Err(_) = result {
            error("Failed to write audio panel SAI stream").await;
            continue;
        }
    }
}

#[instrumented(TaskId::FrontPanelAudioControl)]
#[embassy_executor::task]
async fn control_task(audio: &'static mut Audio) {
    let mut mode_rcv = MODE.receiver().unwrap();
    let mut volume_rcv = VOLUME.receiver().unwrap();
    let mut mic_rcv = MICROPHONE.receiver().unwrap();
    loop {
        match select3(mode_rcv.changed(), volume_rcv.changed(), mic_rcv.changed()).await {
            Either3::First(mode) => {
                if let Err(e) = audio.set_mode(mode).await {
                    error(e).await;
                }
            }
            Either3::Second(volume) => {
                let percent = (volume.raw().max(0) as u32 * 100 / 1000) as u8;
                if let Err(_) = audio.set_volume(percent).await {
                    error("Failed to set audio volume").await;
                }
            }
            Either3::Third(mic) => {
                let percent = (mic.raw().max(0) as u32 * 100 / 1000) as u8;
                if let Err(_) = audio.set_mic_gain(percent).await {
                    error("Failed to set mic gain").await;
                }
            }
        }
    }
}
