use core::fmt::Write;

use embedded_graphics::{
    mono_font::{ascii::{FONT_6X10, FONT_10X20}, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::Text,
};
use heapless::String;

use crate::state::input::{IfGainMode, Mode, RadioState, RfGainMode, TransmitMode};
use crate::ui::{meter_bar, BLACK, BLUE, BRIGHT_GREEN, CYAN, DARK_GRAY, DIM_WHITE, GRAY, GREEN, ORANGE, RED, WHITE, YELLOW};

const DBM_MIN: i8 = -120;
const DBM_S9: i8 = -73;
const DBM_MAX: i8 = -13;

const ACCUMULATOR_MAX: i16 = 1000;

pub fn render(target: &mut impl DrawTarget<Color = Rgb565>, state: &RadioState) {
    let _ = Rectangle::new(Point::zero(), Size::new(240, 135))
        .into_styled(PrimitiveStyle::with_fill(BLACK))
        .draw(target);

    draw_status_bar(target, state);
    draw_frequency(target, state.frequency);
    draw_info_line(target, state);
    draw_smeter(target, state.rssi_dbm);
    draw_vol_sql(target, state);
}

fn band_label(band: u8) -> &'static str {
    match band {
        0 => "160m",
        1 => "80m",
        2 => "40m",
        3 => "30m",
        4 => "20m",
        5 => "17m",
        6 => "15m",
        7 => "12m",
        8 => "10m",
        _ => "??m",
    }
}

fn cursor_highlight_color(state: &RadioState, index: u8) -> Option<Rgb565> {
    if state.cursor_index != index {
        return None;
    }
    if state.cursor_editing {
        Some(YELLOW)
    } else {
        Some(CYAN)
    }
}

fn draw_cursor_outline(target: &mut impl DrawTarget<Color = Rgb565>, x: i32, y: i32, w: u32, h: u32, color: Rgb565) {
    let _ = Rectangle::new(Point::new(x - 1, y), Size::new(w + 2, h))
        .into_styled(PrimitiveStyle::with_stroke(color, 1))
        .draw(target);
}

fn draw_status_bar(target: &mut impl DrawTarget<Color = Rgb565>, state: &RadioState) {
    let _ = Rectangle::new(Point::new(0, 0), Size::new(240, 14))
        .into_styled(PrimitiveStyle::with_fill(DARK_GRAY))
        .draw(target);

    let style = MonoTextStyle::new(&FONT_6X10, DIM_WHITE);
    let mode_style = MonoTextStyle::new(&FONT_6X10, WHITE);

    let mut x: i32 = 2;

    let bl = band_label(state.band);
    let bl_x = x;
    let bl_w = (bl.len() as u32) * 6;
    Text::new(bl, Point::new(x, 10), mode_style).draw(target).ok();
    if let Some(color) = cursor_highlight_color(state, 0) {
        draw_cursor_outline(target, bl_x, 0, bl_w, 14, color);
    }
    x += bl_w as i32 + 4;

    let tx_mode = match state.transmit_mode {
        TransmitMode::Usb => "USB",
        TransmitMode::Lsb => "LSB",
        TransmitMode::Cw => "CW",
        TransmitMode::Am => "AM",
    };
    let tm_x = x;
    let tm_w = (tx_mode.len() as u32) * 6;
    Text::new(tx_mode, Point::new(x, 10), mode_style).draw(target).ok();
    if let Some(color) = cursor_highlight_color(state, 1) {
        draw_cursor_outline(target, tm_x, 0, tm_w, 14, color);
    }
    x += tm_w as i32 + 4;

    let agc = match state.agc_mode {
        IfGainMode::Manual => "AGC:M",
        IfGainMode::AgcFast => "AGC:F",
        IfGainMode::AgcSlow => "AGC:S",
    };
    let agc_x = x;
    let agc_w = (agc.len() as u32) * 6;
    Text::new(agc, Point::new(x, 10), style).draw(target).ok();
    if let Some(color) = cursor_highlight_color(state, 2) {
        draw_cursor_outline(target, agc_x, 0, agc_w, 14, color);
    }
    x += agc_w as i32 + 4;

    let rf = match state.rf_gain_mode {
        RfGainMode::Attenuator => "ATT",
        RfGainMode::Normal => "",
        RfGainMode::RfSingle => "PRE1",
        RfGainMode::RfDouble => "PRE2",
    };
    if !rf.is_empty() {
        let rf_x = x;
        let rf_w = (rf.len() as u32) * 6;
        Text::new(rf, Point::new(x, 10), style).draw(target).ok();
        if let Some(color) = cursor_highlight_color(state, 3) {
            draw_cursor_outline(target, rf_x, 0, rf_w, 14, color);
        }
        x += rf_w as i32 + 4;
    } else if let Some(color) = cursor_highlight_color(state, 3) {
        let rf_w = 18u32;
        draw_cursor_outline(target, x, 0, rf_w, 14, color);
        x += rf_w as i32 + 4;
    }

    if state.nb_enabled {
        let nb_x = x;
        let nb_w = 12u32;
        Text::new("NB", Point::new(x, 10), style).draw(target).ok();
        if let Some(color) = cursor_highlight_color(state, 4) {
            draw_cursor_outline(target, nb_x, 0, nb_w, 14, color);
        }
    } else if let Some(color) = cursor_highlight_color(state, 4) {
        draw_cursor_outline(target, x, 0, 12, 14, color);
    }

    let (mode_label, mode_color) = match state.mode {
        Mode::StandBy => ("STBY", GRAY),
        Mode::WarmUp => ("WARM", ORANGE),
        Mode::Rx => ("RX", BRIGHT_GREEN),
        Mode::Tx => ("TX", RED),
    };
    let mode_rx_style = MonoTextStyle::new(&FONT_6X10, mode_color);
    let mode_x = 240 - (mode_label.len() as i32) * 6 - 2;
    Text::new(mode_label, Point::new(mode_x, 10), mode_rx_style).draw(target).ok();
}

fn draw_frequency(target: &mut impl DrawTarget<Color = Rgb565>, frequency: u32) {
    let mhz = frequency / 1_000_000;
    let khz = (frequency % 1_000_000) / 1_000;
    let hz = frequency % 1_000;

    let mut freq_str: String<16> = String::new();
    let _ = write!(freq_str, "{}.{:03}.{:03}", mhz, khz, hz);

    let freq_style = MonoTextStyle::new(&FONT_10X20, WHITE);
    let text_width = freq_str.len() as i32 * 10;
    let freq_x = (240 - text_width) / 2;
    Text::new(&freq_str, Point::new(freq_x, 40), freq_style).draw(target).ok();

    let unit_style = MonoTextStyle::new(&FONT_6X10, GRAY);
    Text::new("MHz", Point::new(freq_x + text_width + 4, 40), unit_style).draw(target).ok();
}

fn draw_info_line(target: &mut impl DrawTarget<Color = Rgb565>, state: &RadioState) {
    let y: i32 = 70;
    let style = MonoTextStyle::new(&FONT_6X10, DIM_WHITE);

    let mut x: i32 = 2;

    if state.clarifier_mode != 0 {
        let label = if state.clarifier_mode == 1 { "RIT" } else { "XIT" };
        let offset_from_center = state.clarifier_raw as i32 - (ACCUMULATOR_MAX as i32 / 2);
        let offset_hz = (offset_from_center * 5000) / (ACCUMULATOR_MAX as i32 / 2);
        let mut rit_str: String<16> = String::new();
        if offset_hz >= 0 {
            let _ = write!(rit_str, "{} +{}.{:02}k", label, offset_hz / 1000, (offset_hz % 1000) / 10);
        } else {
            let abs_hz = -offset_hz;
            let _ = write!(rit_str, "{} -{}.{:02}k", label, abs_hz / 1000, (abs_hz % 1000) / 10);
        }
        Text::new(&rit_str, Point::new(x, y), style).draw(target).ok();
        x += (rit_str.len() as i32) * 6 + 6;
    }

    let mut bw_str: String<10> = String::new();
    if state.filter_bw_hz >= 1000 {
        let _ = write!(bw_str, "BW:{}.{}k", state.filter_bw_hz / 1000, (state.filter_bw_hz % 1000) / 100);
    } else {
        let _ = write!(bw_str, "BW:{}Hz", state.filter_bw_hz);
    }
    let bw_x = x;
    let bw_w = (bw_str.len() as u32) * 6;
    Text::new(&bw_str, Point::new(x, y), style).draw(target).ok();
    if let Some(color) = cursor_highlight_color(state, 5) {
        draw_cursor_outline(target, bw_x, y - 10, bw_w, 14, color);
    }
    x += bw_w as i32 + 6;

    let pwr_percent = state.rf_power_centipercent / 100;
    let mut pwr_str: String<10> = String::new();
    let _ = write!(pwr_str, "PWR:{}%", pwr_percent);
    let pwr_x = x;
    let pwr_w = (pwr_str.len() as u32) * 6;
    Text::new(&pwr_str, Point::new(x, y), style).draw(target).ok();
    if let Some(color) = cursor_highlight_color(state, 6) {
        draw_cursor_outline(target, pwr_x, y - 10, pwr_w, 14, color);
    }
}

fn dbm_to_bar(dbm: i8) -> u16 {
    let clamped = dbm.clamp(DBM_MIN, DBM_MAX);
    ((clamped as i32 - DBM_MIN as i32) * 4095 / (DBM_MAX as i32 - DBM_MIN as i32)) as u16
}

fn s9_bar() -> u16 {
    dbm_to_bar(DBM_S9)
}

fn draw_smeter(target: &mut impl DrawTarget<Color = Rgb565>, dbm: i8) {
    let bar_y: i32 = 80;
    let bar_height: u32 = 16;
    let bar_x: i32 = 4;
    let bar_width: u32 = 160;

    let value = dbm_to_bar(dbm);

    meter_bar::draw_gradient_meter(
        target,
        bar_x, bar_y, bar_width, bar_height,
        value, 4095,
        s9_bar(),
        GREEN, YELLOW,
        DARK_GRAY,
    );

    let readout_style = MonoTextStyle::new(&FONT_6X10, WHITE);
    let dim_style = MonoTextStyle::new(&FONT_6X10, DIM_WHITE);

    let mut dbm_str: String<10> = String::new();
    let _ = write!(dbm_str, "{}dBm", dbm);
    Text::new(&dbm_str, Point::new(168, bar_y + 11), dim_style).draw(target).ok();

    let mut s_str: String<8> = String::new();
    if dbm <= DBM_S9 {
        let s_unit = ((dbm as i32 - DBM_MIN as i32) * 9) / (DBM_S9 as i32 - DBM_MIN as i32);
        let s_unit = s_unit.clamp(0, 9);
        let _ = write!(s_str, "S{}", s_unit);
    } else {
        let over = dbm as i32 - DBM_S9 as i32;
        let over = (over / 10) * 10;
        let _ = write!(s_str, "S9+{}", over);
    }
    Text::new(&s_str, Point::new(218, bar_y + 11), readout_style).draw(target).ok();
}

fn draw_vol_sql(target: &mut impl DrawTarget<Color = Rgb565>, state: &RadioState) {
    let y: i32 = 108;
    let bar_height: u32 = 12;
    let label_style = MonoTextStyle::new(&FONT_6X10, DIM_WHITE);

    Text::new("VOL", Point::new(2, y + 10), label_style).draw(target).ok();
    let vol_value = state.volume_raw.max(0) as u16;
    meter_bar::draw_meter_bar(
        target,
        24, y, 88, bar_height,
        vol_value, ACCUMULATOR_MAX as u16,
        BLUE, DARK_GRAY,
    );
    if let Some(color) = cursor_highlight_color(state, 7) {
        draw_cursor_outline(target, 1, y, 112, bar_height + 2, color);
    }

    Text::new("SQL", Point::new(120, y + 10), label_style).draw(target).ok();
    let sql_value = state.squelch_raw.max(0) as u16;
    meter_bar::draw_meter_bar(
        target,
        142, y, 88, bar_height,
        sql_value, ACCUMULATOR_MAX as u16,
        GREEN, DARK_GRAY,
    );
    if let Some(color) = cursor_highlight_color(state, 8) {
        draw_cursor_outline(target, 119, y, 112, bar_height + 2, color);
    }
}
