use common::error::error;
use embassy_executor::Spawner;
use embassy_stm32::peripherals as stm_peripherals;
use embassy_stm32::sai::Sai;
use static_cell::StaticCell;

use crate::{
    app::events::{AUDIO_BUFFER_TX, MODE},
    consts::AUDIO_BUFFER_SIZE,
    main_board::{events::AUDIO_RX_BUFFER, modules::audio_panel::AudioPanel},
    runtime_stats::TaskId,
};
use druzhba_macros::instrumented;

type SaiTx = Sai<'static, stm_peripherals::SAI1, u16>;
type SaiRx = Sai<'static, stm_peripherals::SAI1, u16>;

static AUDIO_PANEL: StaticCell<AudioPanel> = StaticCell::new();

pub async fn create_tasks(spawner: Spawner, audio_panel: AudioPanel) {
    let audio_panel = AUDIO_PANEL.init(audio_panel);
    let (sai_tx, sai_rx) = audio_panel.split_sai();
    spawner.must_spawn(audio_panel_sai_rx_task(sai_rx));
    spawner.must_spawn(audio_panel_sai_tx_task(sai_tx));
    spawner.must_spawn(control_task(audio_panel));
}

#[embassy_executor::task]
async fn audio_panel_sai_rx_task(sai_rx: &'static mut SaiRx) {
    let mut buffer: [u16; AUDIO_BUFFER_SIZE] = [0u16; AUDIO_BUFFER_SIZE];

    loop {
        loop {
            let result = sai_rx.read(&mut buffer).await;
            if let Err(_) = result {
                error("Failed to read audio panel SAI stream").await;
                continue;
            }
            AUDIO_RX_BUFFER.sender().send(buffer);
        }
    }
}

#[instrumented(TaskId::AudioPanelSaiTx)]
#[embassy_executor::task]
async fn audio_panel_sai_tx_task(sai_tx: &'static mut SaiTx) {
    let mut tx_rcv = AUDIO_BUFFER_TX.receiver().unwrap();
    loop {
        let buffer = tx_rcv.changed().await;
        let result = sai_tx.write(&buffer).await;
        if let Err(_) = result {
            error("Failed to write audio panel SAI stream").await;
            continue;
        }
    }
}

#[instrumented(TaskId::AudioPanelControl)]
#[embassy_executor::task]
async fn control_task(audio_panel: &'static mut AudioPanel) {
    let mut mode_rcv = MODE.receiver().unwrap();
    loop {
        let mode = mode_rcv.changed().await;

        if let Err(e) = audio_panel.set_mode(mode).await {
            error(e).await;
        }
    }
}
