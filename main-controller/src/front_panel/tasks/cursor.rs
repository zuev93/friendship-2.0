use embassy_futures::select::{select, Either};
use embassy_time::Instant;

use common::protocol_types::MenuCommand;

use crate::app::{
    events::{BAND, FILTER, IF_GAIN_MODE, NB_ENABLED, RF_GAIN_MODE, TRANSMIT_MODE},
    tasks::arbiters::{
        filter::{FilterCommand, FILTER_CMD},
        frequency::{BandCommand, BAND_CMD},
        if_gain_mode::{IfGainModeCommand, IF_GAIN_MODE_CMD},
        nb::{NbCommand, NB_CMD},
        rf_gain_mode::{RfGainModeCommand, RF_GAIN_MODE_CMD},
        rf_power::RF_POWER_CMD,
        squelch::{SquelchCommand, SQUELCH_CMD},
        transmit_mode::{TransmitModeCommand, TRANSMIT_MODE_CMD},
        volume::VOLUME_CMD,
    },
    types::{Band, FilterType, IfGainMode, RfGainMode, TransmitMode},
};
use crate::front_panel::events::{
    CursorCommand, CursorState, CURSOR_CMD, CURSOR_STATE, MENU_CMD_QUEUE, MENU_ENCODER_EVENTS,
};

const ITEM_COUNT: u8 = 9;
const LONG_PRESS_MS: u64 = 500;
const STEP_SIZE: i16 = 4;

enum CursorMode {
    Browsing,
    Editing,
    Menu,
}

enum SavedValue {
    None,
    Band(Band),
    TransmitMode(TransmitMode),
    AgcMode(IfGainMode),
    RfGainMode(RfGainMode),
    NbEnabled(bool),
    Filter(FilterType),
    Continuous,
}

#[embassy_executor::task]
pub async fn cursor_task() {
    let mut mode = CursorMode::Browsing;
    let mut cursor_index: u8 = 0;
    let mut press_time = Instant::now();
    let mut saved_value = SavedValue::None;
    let mut accumulated_delta: i16 = 0;
    let mut menu_depth: u8 = 0;

    publish_cursor(cursor_index, false);

    loop {
        match mode {
            CursorMode::Browsing => {
                match select(CURSOR_CMD.receive(), MENU_ENCODER_EVENTS.receive()).await {
                    Either::First(cmd) => match cmd {
                        CursorCommand::OkPress => {
                            press_time = Instant::now();
                        }
                        CursorCommand::OkRelease => {
                            let held_ms = Instant::now().duration_since(press_time).as_millis();
                            if held_ms >= LONG_PRESS_MS {
                                mode = CursorMode::Menu;
                                menu_depth = 0;
                                MENU_CMD_QUEUE.send(MenuCommand::Ok).await;
                                publish_cursor(0xFF, false);
                            } else {
                                saved_value = snapshot_value(cursor_index);
                                accumulated_delta = 0;
                                mode = CursorMode::Editing;
                                publish_cursor(cursor_index, true);
                            }
                        }
                        CursorCommand::Cancel => {}
                    },
                    Either::Second(delta) => {
                        if delta > 0 {
                            if cursor_index < ITEM_COUNT - 1 {
                                cursor_index += 1;
                            }
                        } else if cursor_index > 0 {
                            cursor_index -= 1;
                        }
                        publish_cursor(cursor_index, false);
                    }
                }
            }
            CursorMode::Editing => {
                match select(CURSOR_CMD.receive(), MENU_ENCODER_EVENTS.receive()).await {
                    Either::First(cmd) => match cmd {
                        CursorCommand::OkPress | CursorCommand::OkRelease => {
                            mode = CursorMode::Browsing;
                            saved_value = SavedValue::None;
                            publish_cursor(cursor_index, false);
                        }
                        CursorCommand::Cancel => {
                            revert_value(&saved_value, accumulated_delta, cursor_index);
                            mode = CursorMode::Browsing;
                            saved_value = SavedValue::None;
                            publish_cursor(cursor_index, false);
                        }
                    },
                    Either::Second(delta) => {
                        dispatch_edit(cursor_index, delta);
                        accumulated_delta += delta as i16 * STEP_SIZE;
                    }
                }
            }
            CursorMode::Menu => {
                match select(CURSOR_CMD.receive(), MENU_ENCODER_EVENTS.receive()).await {
                    Either::First(cmd) => match cmd {
                        CursorCommand::OkPress => {
                            menu_depth += 1;
                            MENU_CMD_QUEUE.send(MenuCommand::Ok).await;
                        }
                        CursorCommand::OkRelease => {}
                        CursorCommand::Cancel => {
                            if menu_depth == 0 {
                                MENU_CMD_QUEUE.send(MenuCommand::Cancel).await;
                                mode = CursorMode::Browsing;
                                publish_cursor(cursor_index, false);
                            } else {
                                menu_depth -= 1;
                                MENU_CMD_QUEUE.send(MenuCommand::Cancel).await;
                            }
                        }
                    },
                    Either::Second(delta) => {
                        MENU_CMD_QUEUE.send(MenuCommand::Scroll(delta)).await;
                    }
                }
            }
        }
    }
}

fn publish_cursor(index: u8, editing: bool) {
    CURSOR_STATE.sender().send(CursorState {
        index,
        editing,
    });
}

fn snapshot_value(index: u8) -> SavedValue {
    match index {
        0 => BAND.try_get().map(SavedValue::Band).unwrap_or(SavedValue::None),
        1 => TRANSMIT_MODE
            .try_get()
            .map(SavedValue::TransmitMode)
            .unwrap_or(SavedValue::None),
        2 => IF_GAIN_MODE
            .try_get()
            .map(SavedValue::AgcMode)
            .unwrap_or(SavedValue::None),
        3 => RF_GAIN_MODE
            .try_get()
            .map(SavedValue::RfGainMode)
            .unwrap_or(SavedValue::None),
        4 => NB_ENABLED
            .try_get()
            .map(SavedValue::NbEnabled)
            .unwrap_or(SavedValue::None),
        5 => FILTER
            .try_get()
            .map(SavedValue::Filter)
            .unwrap_or(SavedValue::None),
        6 | 7 | 8 => SavedValue::Continuous,
        _ => SavedValue::None,
    }
}

fn dispatch_edit(index: u8, delta: i8) {
    match index {
        0 => {
            if delta > 0 {
                BAND_CMD.signal(BandCommand::Up);
            } else {
                BAND_CMD.signal(BandCommand::Down);
            }
        }
        1 => {
            TRANSMIT_MODE_CMD.signal(TransmitModeCommand::Cycle);
        }
        2 => {
            IF_GAIN_MODE_CMD.signal(IfGainModeCommand::Toggle);
        }
        3 => {
            RF_GAIN_MODE_CMD.signal(RfGainModeCommand::Cycle);
        }
        4 => {
            NB_CMD.signal(NbCommand::Toggle);
        }
        5 => {
            FILTER_CMD.signal(FilterCommand::Cycle);
        }
        6 => {
            RF_POWER_CMD.signal(delta as i16 * STEP_SIZE);
        }
        7 => {
            VOLUME_CMD.signal(delta as i16 * STEP_SIZE);
        }
        8 => {
            SQUELCH_CMD.signal(SquelchCommand::Delta(delta as i16 * STEP_SIZE));
        }
        _ => {}
    }
}

fn revert_value(saved: &SavedValue, accumulated_delta: i16, cursor_index: u8) {
    match saved {
        SavedValue::Band(band) => {
            BAND_CMD.signal(BandCommand::Set(*band));
        }
        SavedValue::TransmitMode(tm) => {
            TRANSMIT_MODE_CMD.signal(TransmitModeCommand::Set(*tm));
        }
        SavedValue::AgcMode(agc) => {
            IF_GAIN_MODE_CMD.signal(IfGainModeCommand::Set(*agc));
        }
        SavedValue::RfGainMode(rf) => {
            RF_GAIN_MODE_CMD.signal(RfGainModeCommand::Set(*rf));
        }
        SavedValue::NbEnabled(nb) => {
            NB_CMD.signal(NbCommand::Set(*nb));
        }
        SavedValue::Filter(f) => {
            FILTER_CMD.signal(FilterCommand::Set(*f));
        }
        SavedValue::Continuous => match cursor_index {
            6 => RF_POWER_CMD.signal(-accumulated_delta),
            7 => VOLUME_CMD.signal(-accumulated_delta),
            8 => SQUELCH_CMD.signal(SquelchCommand::Delta(-accumulated_delta)),
            _ => {}
        },
        SavedValue::None => {}
    }
}
