use core::fmt::Write;

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::Text,
};
use heapless::String;

use crate::state::input::{IfGainMode, Mode, RadioState, RfGainMode, TransmitMode};
use crate::ui::{
    meter_bar, BLACK, BLUE, BRIGHT_GREEN, CYAN, DARK_GRAY, DIM_WHITE, GRAY, GREEN, ORANGE, RED,
    WHITE, YELLOW,
};

const DBM_MIN: i8 = -120;
const DBM_S9: i8 = -73;
const DBM_MAX: i8 = -13;

const MAX_POWER_MW: u16 = 50_000;
const MAX_SWR: u16 = 500;
const SWR_WARN: u16 = 200;

const BAR_X: i32 = 2;
const BAR_WIDTH: u32 = 170;
const BAR_HEIGHT: u32 = 14;

pub fn render(target: &mut impl DrawTarget<Color = Rgb565>, state: &RadioState, peak_dbm: i8) {
    let _ = Rectangle::new(Point::zero(), Size::new(240, 135))
        .into_styled(PrimitiveStyle::with_fill(BLACK))
        .draw(target);

    draw_status_bar(target, state);
    draw_scale(target);
    draw_smeter_bar(target, state.rssi_dbm, peak_dbm);
    draw_smeter_readout(target, state.rssi_dbm);
    draw_power_bar(target, state.forward_power_mw, state.rf_power_centipercent);
    draw_swr_bar(target, state.vswr_x100);
    draw_detail_bar(target, state.forward_power_mw, state.vswr_x100);
}

fn dbm_to_bar(dbm: i8) -> u16 {
    let clamped = dbm.clamp(DBM_MIN, DBM_MAX);
    ((clamped as i32 - DBM_MIN as i32) * 4095 / (DBM_MAX as i32 - DBM_MIN as i32)) as u16
}

fn s9_bar() -> u16 {
    dbm_to_bar(DBM_S9)
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

fn draw_cursor_outline(
    target: &mut impl DrawTarget<Color = Rgb565>,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    color: Rgb565,
) {
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

    let tx_mode = match state.transmit_mode {
        TransmitMode::Usb => "USB",
        TransmitMode::Lsb => "LSB",
        TransmitMode::Cw => "CW",
        TransmitMode::Am => "AM",
    };
    let tm_w = (tx_mode.len() as u32) * 6;
    Text::new(tx_mode, Point::new(2, 10), mode_style)
        .draw(target)
        .ok();
    if let Some(color) = cursor_highlight_color(state, 1) {
        draw_cursor_outline(target, 2, 0, tm_w, 14, color);
    }

    let agc = match state.agc_mode {
        IfGainMode::Manual => "AGC:MAN",
        IfGainMode::AgcFast => "AGC:FAST",
        IfGainMode::AgcSlow => "AGC:SLOW",
    };
    let agc_w = (agc.len() as u32) * 6;
    Text::new(agc, Point::new(30, 10), style).draw(target).ok();
    if let Some(color) = cursor_highlight_color(state, 2) {
        draw_cursor_outline(target, 30, 0, agc_w, 14, color);
    }

    let mut bw_str: String<8> = String::new();
    if state.filter_bw_hz >= 1000 {
        let _ = write!(
            bw_str,
            "{}.{}k",
            state.filter_bw_hz / 1000,
            (state.filter_bw_hz % 1000) / 100
        );
    } else {
        let _ = write!(bw_str, "{}Hz", state.filter_bw_hz);
    }
    let bw_w = (bw_str.len() as u32) * 6;
    Text::new(&bw_str, Point::new(96, 10), style)
        .draw(target)
        .ok();
    if let Some(color) = cursor_highlight_color(state, 5) {
        draw_cursor_outline(target, 96, 0, bw_w, 14, color);
    }

    let rf = match state.rf_gain_mode {
        RfGainMode::Attenuator => "ATT",
        RfGainMode::Normal => "",
        RfGainMode::RfSingle => "PRE1",
        RfGainMode::RfDouble => "PRE2",
    };
    if !rf.is_empty() {
        let rf_w = (rf.len() as u32) * 6;
        Text::new(rf, Point::new(138, 10), style).draw(target).ok();
        if let Some(color) = cursor_highlight_color(state, 3) {
            draw_cursor_outline(target, 138, 0, rf_w, 14, color);
        }
    } else if let Some(color) = cursor_highlight_color(state, 3) {
        draw_cursor_outline(target, 138, 0, 18, 14, color);
    }

    let (mode_label, mode_color) = match state.mode {
        Mode::StandBy => ("STBY", GRAY),
        Mode::WarmUp => ("WARM", ORANGE),
        Mode::Rx => ("RX", BRIGHT_GREEN),
        Mode::Tx => ("TX", RED),
    };
    let mode_rx_style = MonoTextStyle::new(&FONT_6X10, mode_color);
    Text::new(mode_label, Point::new(210, 10), mode_rx_style)
        .draw(target)
        .ok();
}

fn draw_scale(target: &mut impl DrawTarget<Color = Rgb565>) {
    let style = MonoTextStyle::new(&FONT_6X10, GRAY);

    let labels: &[(i32, &str)] = &[
        (0, "1"),
        (20, "3"),
        (41, "5"),
        (61, "7"),
        (82, "9"),
        (107, "+20"),
        (132, "+40"),
        (157, "+60"),
    ];
    let base_x = BAR_X;
    for &(offset, label) in labels {
        Text::new(label, Point::new(base_x + offset, 26), style)
            .draw(target)
            .ok();
    }

    let tick_y: i32 = 28;
    let _ = Rectangle::new(Point::new(BAR_X, tick_y), Size::new(BAR_WIDTH, 1))
        .into_styled(PrimitiveStyle::with_fill(GRAY))
        .draw(target);
}

fn draw_smeter_bar(target: &mut impl DrawTarget<Color = Rgb565>, dbm: i8, peak_dbm: i8) {
    let y: i32 = 32;
    let height: u32 = 16;
    let value = dbm_to_bar(dbm);
    let peak = dbm_to_bar(peak_dbm);

    meter_bar::draw_gradient_meter(
        target,
        BAR_X,
        y,
        BAR_WIDTH,
        height,
        value,
        4095,
        s9_bar(),
        GREEN,
        YELLOW,
        DARK_GRAY,
    );

    if peak > value.saturating_add(40) {
        let peak_x = (peak as u32 * BAR_WIDTH) / 4095;
        let _ = Rectangle::new(Point::new(BAR_X + peak_x as i32, y), Size::new(2, height))
            .into_styled(PrimitiveStyle::with_fill(WHITE))
            .draw(target);
    }
}

fn draw_smeter_readout(target: &mut impl DrawTarget<Color = Rgb565>, dbm: i8) {
    let y: i32 = 62;
    let _ = Rectangle::new(Point::new(0, y - 10), Size::new(240, 14))
        .into_styled(PrimitiveStyle::with_fill(BLACK))
        .draw(target);

    let style = MonoTextStyle::new(&FONT_6X10, WHITE);
    let dim_style = MonoTextStyle::new(&FONT_6X10, DIM_WHITE);

    let mut s_str: String<12> = String::new();

    if dbm <= DBM_S9 {
        let s_unit = ((dbm as i32 - DBM_MIN as i32) * 9) / (DBM_S9 as i32 - DBM_MIN as i32);
        let s_unit = s_unit.clamp(0, 9);
        let _ = write!(s_str, "S{}", s_unit);
    } else {
        let over = dbm as i32 - DBM_S9 as i32;
        let over = (over / 10) * 10;
        let _ = write!(s_str, "S9+{}", over);
    }

    Text::new(&s_str, Point::new(130, y), style)
        .draw(target)
        .ok();

    let mut dbm_str: String<12> = String::new();
    let _ = write!(dbm_str, "{} dBm", dbm);
    Text::new(&dbm_str, Point::new(190, y), dim_style)
        .draw(target)
        .ok();
}

fn draw_power_bar(
    target: &mut impl DrawTarget<Color = Rgb565>,
    forward_power_mw: u16,
    rf_power_centipercent: u16,
) {
    let y: i32 = 78;
    let bar_x: i32 = 26;
    let bar_w: u32 = 144;
    let label_style = MonoTextStyle::new(&FONT_6X10, DIM_WHITE);
    let val_style = MonoTextStyle::new(&FONT_6X10, WHITE);

    Text::new("PWR", Point::new(2, y + 10), label_style)
        .draw(target)
        .ok();

    meter_bar::draw_meter_bar(
        target,
        bar_x,
        y,
        bar_w,
        BAR_HEIGHT,
        forward_power_mw,
        MAX_POWER_MW,
        BLUE,
        DARK_GRAY,
    );

    let target_mw = (rf_power_centipercent as u32 * MAX_POWER_MW as u32 / 10000) as u16;
    let target_x = (target_mw as u32 * bar_w) / MAX_POWER_MW as u32;
    let _ = Rectangle::new(
        Point::new(bar_x + target_x as i32, y),
        Size::new(1, BAR_HEIGHT),
    )
    .into_styled(PrimitiveStyle::with_fill(WHITE))
    .draw(target);

    let mut pwr_str: String<10> = String::new();
    let watts = forward_power_mw / 1000;
    let frac = (forward_power_mw % 1000) / 100;
    let _ = write!(pwr_str, "{}.{} W", watts, frac);
    Text::new(&pwr_str, Point::new(176, y + 10), val_style)
        .draw(target)
        .ok();
}

fn draw_swr_bar(target: &mut impl DrawTarget<Color = Rgb565>, vswr_x100: u16) {
    let y: i32 = 96;
    let label_style = MonoTextStyle::new(&FONT_6X10, DIM_WHITE);
    let val_style = MonoTextStyle::new(&FONT_6X10, WHITE);

    Text::new("SWR", Point::new(2, y + 10), label_style)
        .draw(target)
        .ok();

    let bar_color = if vswr_x100 > SWR_WARN { RED } else { GREEN };
    meter_bar::draw_meter_bar(
        target, 26, y, 144, BAR_HEIGHT, vswr_x100, MAX_SWR, bar_color, DARK_GRAY,
    );

    let mut swr_str: String<10> = String::new();
    let whole = vswr_x100 / 100;
    let frac = vswr_x100 % 100;
    if frac < 10 {
        let _ = write!(swr_str, "{}.0{}:1", whole, frac);
    } else {
        let _ = write!(swr_str, "{}.{}:1", whole, frac / 10);
    }
    Text::new(&swr_str, Point::new(176, y + 10), val_style)
        .draw(target)
        .ok();
}

fn draw_detail_bar(
    target: &mut impl DrawTarget<Color = Rgb565>,
    forward_power_mw: u16,
    vswr_x100: u16,
) {
    let y: i32 = 125;
    let style = MonoTextStyle::new(&FONT_6X10, DIM_WHITE);

    let _ = Rectangle::new(Point::new(0, y - 10), Size::new(240, 14))
        .into_styled(PrimitiveStyle::with_fill(DARK_GRAY))
        .draw(target);

    let mut fwd_str: String<12> = String::new();
    let _ = write!(fwd_str, "FWD:{}W", forward_power_mw / 1000);
    Text::new(&fwd_str, Point::new(2, y), style)
        .draw(target)
        .ok();

    let reflected_mw = if vswr_x100 > 100 {
        let gamma_num = (vswr_x100 as u32).saturating_sub(100);
        let gamma_den = vswr_x100 as u32 + 100;
        ((forward_power_mw as u32 * gamma_num * gamma_num) / (gamma_den * gamma_den)) as u16
    } else {
        0
    };
    let mut ref_str: String<12> = String::new();
    let ref_w = reflected_mw / 1000;
    let ref_frac = (reflected_mw % 1000) / 100;
    let _ = write!(ref_str, "REF:{}.{}W", ref_w, ref_frac);
    Text::new(&ref_str, Point::new(72, y), style)
        .draw(target)
        .ok();
}
