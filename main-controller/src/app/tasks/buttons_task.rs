use crate::app::{
    events::BUTTON_BEEP,
    tasks::arbiters::{
        anf::{AnfCommand, ANF_CMD},
        audio_agc::{AudioAgcCommand, AUDIO_AGC_CMD},
        clarifier_mode::{ClarifierModeCommand, CLARIFIER_MODE_CMD},
        cw_peak::{CwPeakCommand, CW_PEAK_CMD},
        dsp_filter::{DspFilterCommand, DSP_FILTER_CMD},
        filter::{FilterCommand, FILTER_CMD},
        frequency::{BandCommand, BAND_CMD},
        if_gain_mode::{IfGainModeCommand, IF_GAIN_MODE_CMD},
        mode::{ModeCommand, MODE_CMD},
        nb::{NbCommand, NB_CMD},
        nr::{NrCommand, NR_CMD},
        rf_gain_mode::{RfGainModeCommand, RF_GAIN_MODE_CMD},
        rx_eq::{RxEqCommand, RX_EQ_CMD},
        scan::{ScanCommand, SCAN_CMD},
        squelch::{SquelchCommand, SQUELCH_CMD},
        tone::{ToneCommand, TONE_CMD},
        transmit_mode::{TransmitModeCommand, TRANSMIT_MODE_CMD},
        tx_eq::{TxEqCommand, TX_EQ_CMD},
        vox::{VoxCommand, VOX_CMD},
        cw_keyer::{CwKeyerCommand, CW_KEYER_CMD},
    },
};
use crate::front_panel::{
    events::{ButtonEvent, BUTTON_EVENTS, MENU_CMD_QUEUE},
    types::ButtonFunction,
};
use common::protocol_types::MenuCommand;
use embassy_time::{Duration, Instant};
use heapless::index_map::FnvIndexMap;

const DEBOUNCE_MS: u64 = 50;

fn debounce(
    last_event_time: &mut FnvIndexMap<(ButtonFunction, bool), Instant, 16>,
    function: ButtonFunction,
    is_press: bool,
) -> bool {
    let now = Instant::now();
    let key = (function, is_press);

    if let Some(&last_time) = last_event_time.get(&key) {
        if now.duration_since(last_time) < Duration::from_millis(DEBOUNCE_MS) {
            return false;
        }
    }

    last_event_time.insert(key, now).ok();
    true
}

#[embassy_executor::task]
pub async fn buttons_task() {
    let button_rx = BUTTON_EVENTS.receiver();
    let mut last_event_time: FnvIndexMap<(ButtonFunction, bool), Instant, 16> = FnvIndexMap::new();

    loop {
        let event = button_rx.receive().await;
        let (function, is_press) = match &event {
            ButtonEvent::Press(f) => (*f, true),
            ButtonEvent::Release(f) => (*f, false),
        };
        if !debounce(&mut last_event_time, function, is_press) {
            continue;
        }
        if is_press {
            BUTTON_BEEP.sender().send(());
        }
        match event {
            ButtonEvent::Release(ButtonFunction::Power) => {
                MODE_CMD.signal(ModeCommand::PowerToggle);
            }
            ButtonEvent::Press(ButtonFunction::Transmit) => {
                SCAN_CMD.signal(ScanCommand::Stop);
                MODE_CMD.signal(ModeCommand::TransmitPress);
            }
            ButtonEvent::Release(ButtonFunction::Transmit) => {
                MODE_CMD.signal(ModeCommand::TransmitRelease);
            }
            ButtonEvent::Press(ButtonFunction::Tone) => {
                SCAN_CMD.signal(ScanCommand::Stop);
                MODE_CMD.signal(ModeCommand::TonePress);
                TONE_CMD.signal(ToneCommand::Press);
            }
            ButtonEvent::Release(ButtonFunction::Tone) => {
                MODE_CMD.signal(ModeCommand::ToneRelease);
                TONE_CMD.signal(ToneCommand::Release);
            }
            ButtonEvent::Press(ButtonFunction::TransmitMode) => {
                TRANSMIT_MODE_CMD.signal(TransmitModeCommand::Cycle);
            }
            ButtonEvent::Press(ButtonFunction::Filter) => {
                FILTER_CMD.signal(FilterCommand::Cycle);
            }
            ButtonEvent::Press(ButtonFunction::Rit) => {
                CLARIFIER_MODE_CMD.signal(ClarifierModeCommand::Toggle);
            }
            ButtonEvent::Press(ButtonFunction::RfGain) => {
                RF_GAIN_MODE_CMD.signal(RfGainModeCommand::Cycle);
            }
            ButtonEvent::Press(ButtonFunction::Agc) => {
                IF_GAIN_MODE_CMD.signal(IfGainModeCommand::Toggle);
            }
            ButtonEvent::Press(ButtonFunction::NoiseBlanker) => {
                NB_CMD.signal(NbCommand::Toggle);
            }
            ButtonEvent::Press(ButtonFunction::NoiseReduction) => {
                NR_CMD.signal(NrCommand::Toggle);
            }

            ButtonEvent::Press(ButtonFunction::AutoNotch) => {
                ANF_CMD.signal(AnfCommand::Toggle);
            }
            ButtonEvent::Press(ButtonFunction::CwPeak) => {
                CW_PEAK_CMD.signal(CwPeakCommand::Toggle);
            }
            ButtonEvent::Press(ButtonFunction::AudioAgc) => {
                AUDIO_AGC_CMD.signal(AudioAgcCommand::Cycle);
            }
            ButtonEvent::Press(ButtonFunction::DspFilter) => {
                DSP_FILTER_CMD.signal(DspFilterCommand::Toggle);
            }
            ButtonEvent::Press(ButtonFunction::TxEqualizer) => {
                TX_EQ_CMD.signal(TxEqCommand::Toggle);
            }
            ButtonEvent::Press(ButtonFunction::RxEqualizer) => {
                RX_EQ_CMD.signal(RxEqCommand::Toggle);
            }

            ButtonEvent::Press(ButtonFunction::Vox) => {
                VOX_CMD.signal(VoxCommand::Toggle);
            }

            ButtonEvent::Press(ButtonFunction::Scan) => {
                SCAN_CMD.signal(ScanCommand::Toggle);
            }

            ButtonEvent::Press(ButtonFunction::CwKeyMode) => {
                CW_KEYER_CMD.signal(CwKeyerCommand::CycleMode);
            }
            ButtonEvent::Press(ButtonFunction::CwPaddleSwap) => {
                CW_KEYER_CMD.signal(CwKeyerCommand::ToggleSwap);
            }

            ButtonEvent::Press(ButtonFunction::Power)
            | ButtonEvent::Release(ButtonFunction::TransmitMode)
            | ButtonEvent::Release(ButtonFunction::Filter)
            | ButtonEvent::Release(ButtonFunction::Rit)
            | ButtonEvent::Release(ButtonFunction::RfGain)
            | ButtonEvent::Release(ButtonFunction::Agc)
            | ButtonEvent::Release(ButtonFunction::NoiseBlanker)
            | ButtonEvent::Release(ButtonFunction::NoiseReduction)
            | ButtonEvent::Release(ButtonFunction::AutoNotch)
            | ButtonEvent::Release(ButtonFunction::CwPeak)
            | ButtonEvent::Release(ButtonFunction::AudioAgc)
            | ButtonEvent::Release(ButtonFunction::DspFilter)
            | ButtonEvent::Release(ButtonFunction::TxEqualizer)
            | ButtonEvent::Release(ButtonFunction::RxEqualizer)
            | ButtonEvent::Release(ButtonFunction::Vox)
            | ButtonEvent::Release(ButtonFunction::Scan)
            | ButtonEvent::Release(ButtonFunction::CwKeyMode)
            | ButtonEvent::Release(ButtonFunction::CwPaddleSwap) => {}

            ButtonEvent::Press(ButtonFunction::IcomPtt) => {
                SCAN_CMD.signal(ScanCommand::Stop);
                MODE_CMD.signal(ModeCommand::TransmitPress);
            }
            ButtonEvent::Release(ButtonFunction::IcomPtt) => {
                MODE_CMD.signal(ModeCommand::TransmitRelease);
            }
            ButtonEvent::Press(ButtonFunction::IcomSql) => {
                SQUELCH_CMD.signal(SquelchCommand::Toggle);
            }
            ButtonEvent::Press(ButtonFunction::IcomUpDown) => {
                BAND_CMD.signal(BandCommand::Up);
            }

            ButtonEvent::Release(ButtonFunction::IcomSql)
            | ButtonEvent::Release(ButtonFunction::IcomUpDown) => {}

            ButtonEvent::Press(ButtonFunction::Ok) => {
                MENU_CMD_QUEUE.send(MenuCommand::Ok).await;
            }
            ButtonEvent::Press(ButtonFunction::Cancel) => {
                MENU_CMD_QUEUE.send(MenuCommand::Cancel).await;
            }
            ButtonEvent::Release(ButtonFunction::Cancel)
            | ButtonEvent::Release(ButtonFunction::Ok) => {}
        }
    }
}
