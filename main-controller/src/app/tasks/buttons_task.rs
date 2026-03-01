use crate::app::{
    events::BUTTON_BEEP,
    tasks::arbiters::{
        clarifier_mode::{ClarifierModeCommand, CLARIFIER_MODE_CMD},
        filter::{FilterCommand, FILTER_CMD},
        if_gain_mode::{IfGainModeCommand, IF_GAIN_MODE_CMD},
        mode::{ModeCommand, MODE_CMD},
        nb::{NbCommand, NB_CMD},
        rf_gain_mode::{RfGainModeCommand, RF_GAIN_MODE_CMD},
        tone::{ToneCommand, TONE_CMD},
        transmit_mode::{TransmitModeCommand, TRANSMIT_MODE_CMD},
    },
};
use crate::front_panel::{
    events::{ButtonEvent, BUTTON_EVENTS},
    types::ButtonFunction,
};
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
    let mut last_event_time: FnvIndexMap<(ButtonFunction, bool), Instant, 16> =
        FnvIndexMap::new();

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
            BUTTON_BEEP.signal(());
        }
        match event {
            ButtonEvent::Release(ButtonFunction::Power) => {
                MODE_CMD.signal(ModeCommand::PowerToggle);
            }
            ButtonEvent::Press(ButtonFunction::Transmit) => {
                MODE_CMD.signal(ModeCommand::TransmitPress);
            }
            ButtonEvent::Release(ButtonFunction::Transmit) => {
                MODE_CMD.signal(ModeCommand::TransmitRelease);
            }
            ButtonEvent::Press(ButtonFunction::Tone) => {
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
            _ => {}
        }
    }
}
