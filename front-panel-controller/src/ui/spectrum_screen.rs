use core::fmt::Write;

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::Text,
};
use heapless::String;

use crate::state::input::{SweepStatus, WaterfallLineData, WATERFALL_BINS};
use crate::ui::{BLACK, DARK_GRAY, DIM_WHITE, GRAY, GREEN, ORANGE, WHITE, YELLOW};

const DISPLAY_WIDTH: usize = 240;
const STATUS_BAR_HEIGHT: i32 = 12;
const SPECTRUM_HEIGHT: i32 = 33;
const WATERFALL_HEIGHT: usize = 90;
const SPECTRUM_Y: i32 = STATUS_BAR_HEIGHT;
const WATERFALL_Y: i32 = STATUS_BAR_HEIGHT + SPECTRUM_HEIGHT;

const DBM_MIN: i8 = -120;
const DBM_MAX: i8 = -20;

pub struct WaterfallDisplayBuffer {
    lines: [[i8; WATERFALL_BINS]; WATERFALL_HEIGHT],
    write_index: usize,
    count: usize,
    center_freq: u32,
    span_hz: u32,
    sweep_status: SweepStatus,
}

impl WaterfallDisplayBuffer {
    pub const fn new() -> Self {
        Self {
            lines: [[DBM_MIN; WATERFALL_BINS]; WATERFALL_HEIGHT],
            write_index: 0,
            count: 0,
            center_freq: 7_100_000,
            span_hz: 100_000,
            sweep_status: SweepStatus::Idle,
        }
    }

    pub fn push(&mut self, data: &WaterfallLineData) {
        self.lines[self.write_index] = data.bins;
        self.write_index = (self.write_index + 1) % WATERFALL_HEIGHT;
        if self.count < WATERFALL_HEIGHT {
            self.count += 1;
        }
        self.center_freq = data.center_freq;
        self.span_hz = data.span_hz;
        self.sweep_status = data.sweep_status;
    }

    fn newest_line(&self) -> &[i8; WATERFALL_BINS] {
        let idx = if self.write_index == 0 {
            WATERFALL_HEIGHT - 1
        } else {
            self.write_index - 1
        };
        &self.lines[idx]
    }

    fn line_at(&self, row: usize) -> &[i8; WATERFALL_BINS] {
        let idx = (self.write_index + WATERFALL_HEIGHT - self.count + row) % WATERFALL_HEIGHT;
        &self.lines[idx]
    }
}

pub fn render(target: &mut impl DrawTarget<Color = Rgb565>, buf: &WaterfallDisplayBuffer, stale: bool) {
    let _ = Rectangle::new(Point::zero(), Size::new(240, 135))
        .into_styled(PrimitiveStyle::with_fill(BLACK))
        .draw(target);

    draw_status_bar(target, buf, stale);
    draw_spectrum(target, buf);
    draw_waterfall(target, buf, stale);
}

fn draw_status_bar(target: &mut impl DrawTarget<Color = Rgb565>, buf: &WaterfallDisplayBuffer, stale: bool) {
    let _ = Rectangle::new(Point::new(0, 0), Size::new(240, STATUS_BAR_HEIGHT as u32))
        .into_styled(PrimitiveStyle::with_fill(DARK_GRAY))
        .draw(target);

    let style = MonoTextStyle::new(&FONT_6X10, DIM_WHITE);

    let half_span = buf.span_hz / 2;
    let left_freq = buf.center_freq.saturating_sub(half_span);
    let right_freq = buf.center_freq + half_span;

    let mut left_str: String<12> = String::new();
    format_freq(&mut left_str, left_freq);
    Text::new(&left_str, Point::new(2, 10), style).draw(target).ok();

    let mut center_str: String<12> = String::new();
    format_freq(&mut center_str, buf.center_freq);
    let cx = 120 - (center_str.len() as i32 * 3);
    Text::new(&center_str, Point::new(cx, 10), style).draw(target).ok();

    let mut right_str: String<12> = String::new();
    format_freq(&mut right_str, right_freq);
    let rx = 240 - right_str.len() as i32 * 6 - 10;
    Text::new(&right_str, Point::new(rx, 10), style).draw(target).ok();

    let dot_color = if stale {
        GRAY
    } else {
        match buf.sweep_status {
            SweepStatus::Sweeping => GREEN,
            SweepStatus::Listening => YELLOW,
            SweepStatus::Idle => GRAY,
        }
    };
    let _ = Rectangle::new(Point::new(234, 3), Size::new(4, 4))
        .into_styled(PrimitiveStyle::with_fill(dot_color))
        .draw(target);
}

fn format_freq(s: &mut String<12>, freq_hz: u32) {
    let mhz = freq_hz / 1_000_000;
    let khz = (freq_hz % 1_000_000) / 1000;
    let _ = write!(s, "{}.{:03}", mhz, khz);
}

fn draw_spectrum(target: &mut impl DrawTarget<Color = Rgb565>, buf: &WaterfallDisplayBuffer) {
    let bins = buf.newest_line();
    let center_x = DISPLAY_WIDTH / 2;

    let _ = Rectangle::new(
        Point::new(center_x as i32, SPECTRUM_Y),
        Size::new(1, SPECTRUM_HEIGHT as u32),
    )
    .into_styled(PrimitiveStyle::with_fill(DARK_GRAY))
    .draw(target);

    let mut prev_y: Option<i32> = None;

    for x in 0..DISPLAY_WIDTH {
        let dbm = bins[x];
        let norm = dbm_to_norm(dbm);
        let bar_h = (norm as i32 * (SPECTRUM_HEIGHT - 1)) / 255;
        let y = SPECTRUM_Y + SPECTRUM_HEIGHT - 1 - bar_h;

        let _ = Rectangle::new(
            Point::new(x as i32, y + 1),
            Size::new(1, (SPECTRUM_Y + SPECTRUM_HEIGHT - y - 1) as u32),
        )
        .into_styled(PrimitiveStyle::with_fill(Rgb565::new(0, 12, 0)))
        .draw(target);

        if let Some(py) = prev_y {
            let (from_y, to_y) = if py < y { (py, y) } else { (y, py) };
            for dy in from_y..=to_y {
                let _ = Rectangle::new(Point::new(x as i32, dy), Size::new(1, 1))
                    .into_styled(PrimitiveStyle::with_fill(GREEN))
                    .draw(target);
            }
        } else {
            let _ = Rectangle::new(Point::new(x as i32, y), Size::new(1, 1))
                .into_styled(PrimitiveStyle::with_fill(GREEN))
                .draw(target);
        }

        prev_y = Some(y);
    }
}

fn draw_waterfall(target: &mut impl DrawTarget<Color = Rgb565>, buf: &WaterfallDisplayBuffer, stale: bool) {
    let rows_to_draw = buf.count.min(WATERFALL_HEIGHT);
    let center_x = DISPLAY_WIDTH / 2;

    for row in 0..rows_to_draw {
        let line = buf.line_at(row);
        let y = WATERFALL_Y + row as i32;

        for x in 0..DISPLAY_WIDTH {
            let color = dbm_to_color(line[x]);
            let _ = Rectangle::new(Point::new(x as i32, y), Size::new(1, 1))
                .into_styled(PrimitiveStyle::with_fill(color))
                .draw(target);
        }

        let center_color = if line[center_x] > DBM_MIN + 10 {
            WHITE
        } else {
            DARK_GRAY
        };
        let _ = Rectangle::new(Point::new(center_x as i32, y), Size::new(1, 1))
            .into_styled(PrimitiveStyle::with_fill(center_color))
            .draw(target);
    }

    if stale && rows_to_draw > 0 {
        let stale_y = WATERFALL_Y + rows_to_draw as i32 - 1;
        let _ = Rectangle::new(
            Point::new(0, stale_y),
            Size::new(DISPLAY_WIDTH as u32, 1),
        )
        .into_styled(PrimitiveStyle::with_fill(ORANGE))
        .draw(target);
    }
}

fn dbm_to_norm(dbm: i8) -> u8 {
    let clamped = dbm.clamp(DBM_MIN, DBM_MAX);
    ((clamped as i32 - DBM_MIN as i32) * 255 / (DBM_MAX as i32 - DBM_MIN as i32)) as u8
}

fn dbm_to_color(dbm: i8) -> Rgb565 {
    let n = dbm_to_norm(dbm) as u16;

    if n < 64 {
        let t = n;
        Rgb565::new((t / 4) as u8, (t / 4) as u8, (4 + t / 2) as u8)
    } else if n < 128 {
        let t = n - 64;
        Rgb565::new(0, (t / 2) as u8, (31 - t / 4) as u8)
    } else if n < 192 {
        let t = n - 128;
        Rgb565::new((t / 2) as u8, (31 + t / 2).min(63) as u8, 0)
    } else {
        let t = n - 192;
        Rgb565::new((31).min(16 + t / 4) as u8, (63 - t).max(0) as u8, 0)
    }
}
