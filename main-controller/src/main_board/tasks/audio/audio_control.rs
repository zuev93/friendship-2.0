// use embassy_futures::select::{select6, Either6};

// use crate::{
//     app::events::{
//         CURRENT_MICROPHONE, CURRENT_MODE, CURRENT_SQUELCH, CURRENT_TRANSMIT_MODE, CURRENT_VOLUME,
//     },
//     main_board::{events::CURRENT_RSSI, types::MainBoardMutex},
// };
// use common::error::error;

// #[embassy_executor::task]
// pub async fn audio_control_task(hardware: MainBoardMutex) {
//     loop {
//         match select6(
//             CURRENT_VOLUME.wait(),
//             CURRENT_MICROPHONE.wait(),
//             CURRENT_MODE.wait(),
//             CURRENT_TRANSMIT_MODE.wait(),
//             CURRENT_SQUELCH.wait(),
//             CURRENT_RSSI.wait(),
//         )
//         .await
//         {
//             Either6::First(volume) => {
//                 let volume_percent = ((volume.max(0) as u32 * 100) / 26500) as u8;

//                 let mut hw = hardware.lock().await;
//                 if let Err(e) = hw.audio_control.set_af_volume(volume_percent).await {
//                     error(e).await;
//                 }
//             }

//             Either6::Second(microphone) => {
//                 let mic_gain_percent = ((microphone.max(0) as u32 * 100) / 26500) as u8;

//                 let mut hw = hardware.lock().await;
//                 if let Err(e) = hw.audio_control.set_mic_gain(mic_gain_percent).await {
//                     error(e).await;
//                 }
//             }

//             Either6::Third(mode) => {
//                 let mut hw = hardware.lock().await;
//                 if let Err(e) = hw.audio_control.set_mode(mode).await {
//                     error(e).await;
//                 }
//             }
//             Either6::Fourth(transmit_mode) => {
//                 let mut hw = hardware.lock().await;
//                 if let Err(e) = hw.audio_control.set_transmit_mode(transmit_mode).await {
//                     error(e).await;
//                 }
//             }
//             Either6::Fifth(squelch) => {
//                 let mut hw = hardware.lock().await;
//                 if let Err(e) = hw.audio_control.set_squelch(squelch).await {
//                     error(e).await;
//                 }
//             }
//             Either6::Sixth(rssi) => {
//                 let mut hw = hardware.lock().await;
//                 if let Err(e) = hw.audio_control.set_rssi(rssi).await {
//                     error(e).await;
//                 }
//             }
//         }
//     }
// }
