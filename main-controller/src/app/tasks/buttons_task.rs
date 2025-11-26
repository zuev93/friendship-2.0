use crate::app::{
    events::{
        CURRENT_CLARIFIER_MODE, CURRENT_IF_GAIN_MODE, CURRENT_MODE, CURRENT_RF_GAIN_MODE,
        CURRENT_TRANSMIT_MODE, TONE_ACTIVE,
    },
    types::{ClarifierMode, IfGainMode, Mode, RfGainMode, TransmitMode},
};
use crate::front_panel::{
    events::{ButtonEvent, BUTTON_EVENTS},
    types::ButtonFunction,
};
use embassy_time::{Duration, Instant};
use heapless::index_map::FnvIndexMap;

// TODO move to settings
const DEBOUNCE_MS: u64 = 50; // 50ms debounce time

struct ButtonState {
    mode: Mode,
    transmit_mode: TransmitMode,
    clarifier_mode: ClarifierMode,
    rf_gain_mode: RfGainMode,
    if_gain_mode: IfGainMode,
    last_press_time: FnvIndexMap<ButtonFunction, Instant, 8>,
    last_release_time: FnvIndexMap<ButtonFunction, Instant, 8>,
}

// TODO move to settings
impl ButtonState {
    fn new() -> Self {
        Self {
            mode: Mode::StandBy,
            transmit_mode: TransmitMode::Usb,
            clarifier_mode: ClarifierMode::Off,
            rf_gain_mode: RfGainMode::Normal,
            if_gain_mode: IfGainMode::Manual,
            last_press_time: FnvIndexMap::new(),
            last_release_time: FnvIndexMap::new(),
        }
    }

    fn should_process_press(&mut self, function: ButtonFunction) -> bool {
        let now = Instant::now();

        if let Some(&last_time) = self.last_press_time.get(&function) {
            let elapsed = now.duration_since(last_time);
            if elapsed < Duration::from_millis(DEBOUNCE_MS) {
                return false;
            }
        }

        self.last_press_time.insert(function, now).ok();
        true
    }

    fn should_process_release(&mut self, function: ButtonFunction) -> bool {
        let now = Instant::now();

        if let Some(&last_time) = self.last_release_time.get(&function) {
            let elapsed = now.duration_since(last_time);
            if elapsed < Duration::from_millis(DEBOUNCE_MS) {
                return false;
            }
        }

        self.last_release_time.insert(function, now).ok();
        true
    }
}

#[embassy_executor::task]
pub async fn buttons_task() {
    let button_rx = BUTTON_EVENTS.receiver();
    let mut state = ButtonState::new();

    loop {
        let button_event = button_rx.receive().await;

        match button_event {
            ButtonEvent::Press(function) => {
                if !state.should_process_press(function) {
                    continue;
                }
                crate::app::events::BUTTON_BEEP.signal(());
                handle_button_press(function, &mut state);
            }
            ButtonEvent::Release(function) => {
                if !state.should_process_release(function) {
                    continue;
                }
                handle_button_release(function, &mut state);
            }
        }
    }
}

fn handle_button_press(function: ButtonFunction, state: &mut ButtonState) {
    match function {
        ButtonFunction::Power => {
            // Power button is handled on release
        }
        ButtonFunction::Transmit => {
            if state.mode == Mode::Rx {
                CURRENT_MODE.signal(Mode::Tx);
            }
        }
        ButtonFunction::Tone => {
            if state.mode == Mode::Rx {
                CURRENT_MODE.signal(Mode::Tx);
                TONE_ACTIVE.signal(true);
            }
        }
        ButtonFunction::TransmitMode => {
            // Cycle through SSB -> CW -> AM
            state.transmit_mode = state.transmit_mode.next();
            CURRENT_TRANSMIT_MODE.signal(state.transmit_mode);
        }
        ButtonFunction::Rit => {
            // Toggle RIT on/off
            state.clarifier_mode = state.clarifier_mode.toggle();
            CURRENT_CLARIFIER_MODE.signal(state.clarifier_mode);
        }
        ButtonFunction::RfGain => {
            // Cycle through Attenuator -> Normal -> RF Amp
            state.rf_gain_mode = state.rf_gain_mode.next();
            CURRENT_RF_GAIN_MODE.signal(state.rf_gain_mode);
        }
        ButtonFunction::Agc => {
            // Toggle AGC on/off
            state.if_gain_mode = state.if_gain_mode.toggle();
            CURRENT_IF_GAIN_MODE.signal(state.if_gain_mode);
        }
        ButtonFunction::Cancel => {
            // TODO: Implement cancel/back functionality for UI navigation
        }
        ButtonFunction::Ok => {
            // TODO: Implement OK/select functionality for UI navigation
        }
        ButtonFunction::IcomPtt => {
            // TODO: Implement ICOM PTT functionality
        }
        ButtonFunction::IcomSql => {
            // TODO: Implement ICOM Squelch functionality
        }
        ButtonFunction::IcomUpDown => {
            // TODO: Implement ICOM Up/Down functionality
        }
    }
}

fn handle_button_release(function: ButtonFunction, state: &mut ButtonState) {
    match function {
        ButtonFunction::Power => {
            if state.mode == Mode::StandBy {
                state.mode = Mode::WarmUp;
            } else {
                state.mode = Mode::StandBy;
            }
            CURRENT_MODE.signal(state.mode);
        }
        ButtonFunction::Transmit => {
            if state.mode == Mode::Tx {
                CURRENT_MODE.signal(Mode::Rx);
            }
        }
        ButtonFunction::Tone => {
            if state.mode == Mode::Tx {
                CURRENT_MODE.signal(Mode::Rx);
                TONE_ACTIVE.signal(false);
            }
        }
        ButtonFunction::TransmitMode => {
            // Handled on press
        }
        ButtonFunction::Rit => {
            // Handled on press
        }
        ButtonFunction::RfGain => {
            // Handled on press
        }
        ButtonFunction::Agc => {
            // Handled on press
        }
        ButtonFunction::Cancel => {
            // Handled on press
        }
        ButtonFunction::Ok => {
            // Handled on press
        }
        ButtonFunction::IcomPtt => {
            // Handled on press
        }
        ButtonFunction::IcomSql => {
            // Handled on press
        }
        ButtonFunction::IcomUpDown => {
            // Handled on press
        }
    }
}
