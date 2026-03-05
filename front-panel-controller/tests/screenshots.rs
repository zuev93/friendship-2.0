use embedded_graphics::{pixelcolor::Rgb565, prelude::*};
use image::RgbImage;
use std::path::PathBuf;

use druzhba_common::protocol_types::{CrashInfoCommand, SweepStatus, WATERFALL_BINS};
use druzhba_front_panel_controller::state::input::{
    FatalError, IfGainMode, Mode, RadioState, RfGainMode, TransmitMode, WaterfallLineData,
};
use druzhba_front_panel_controller::state::menu::{MenuItemView, MenuScreen};
use druzhba_front_panel_controller::ui::error_screen;
use druzhba_front_panel_controller::ui::main_screen;
use druzhba_front_panel_controller::ui::menu_screen;
use druzhba_front_panel_controller::ui::meter_screen;
use druzhba_front_panel_controller::ui::spectrum_screen;
use druzhba_front_panel_controller::ui::spectrum_screen::WaterfallDisplayBuffer;

const WIDTH: usize = 240;
const HEIGHT: usize = 135;
const SCALE: u32 = 3;

struct Framebuffer {
    pixels: Box<[Rgb565; WIDTH * HEIGHT]>,
}

impl Framebuffer {
    fn new() -> Self {
        Self {
            pixels: Box::new([Rgb565::BLACK; WIDTH * HEIGHT]),
        }
    }
}

impl DrawTarget for Framebuffer {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>>,
    {
        for embedded_graphics::Pixel(Point { x, y }, color) in pixels {
            if x >= 0 && x < WIDTH as i32 && y >= 0 && y < HEIGHT as i32 {
                self.pixels[y as usize * WIDTH + x as usize] = color;
            }
        }
        Ok(())
    }
}

impl OriginDimensions for Framebuffer {
    fn size(&self) -> Size {
        Size::new(WIDTH as u32, HEIGHT as u32)
    }
}

fn save_png(fb: &Framebuffer, name: &str) {
    let scaled_w = WIDTH as u32 * SCALE;
    let scaled_h = HEIGHT as u32 * SCALE;
    let mut img = RgbImage::new(scaled_w, scaled_h);

    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let c = fb.pixels[y * WIDTH + x];
            let raw = embedded_graphics::pixelcolor::raw::RawU16::from(c).into_inner();
            let r5 = ((raw >> 11) & 0x1F) as u8;
            let g6 = ((raw >> 5) & 0x3F) as u8;
            let b5 = (raw & 0x1F) as u8;
            let r8 = (r5 << 3) | (r5 >> 2);
            let g8 = (g6 << 2) | (g6 >> 4);
            let b8 = (b5 << 3) | (b5 >> 2);

            for sy in 0..SCALE {
                for sx in 0..SCALE {
                    img.put_pixel(
                        x as u32 * SCALE + sx,
                        y as u32 * SCALE + sy,
                        image::Rgb([r8, g8, b8]),
                    );
                }
            }
        }
    }

    let docs_dir = docs_dir();
    std::fs::create_dir_all(&docs_dir).unwrap();
    let path = docs_dir.join(name);
    img.save(&path).unwrap();
    eprintln!("Saved: {}", path.display());
}

fn docs_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("docs")
}

fn rx_state() -> RadioState {
    RadioState {
        band: 3,
        mode: Mode::Rx,
        transmit_mode: TransmitMode::Usb,
        agc_mode: IfGainMode::AgcFast,
        rf_gain_mode: RfGainMode::RfSingle,
        nb_enabled: true,
        clarifier_mode: 1,
        clarifier_raw: 120,
        filter_bw_hz: 2700,
        rf_power_centipercent: 10000,
        frequency: 7_100_000,
        rssi_dbm: -73,
        forward_power_mw: 0,
        vswr_x100: 100,
        volume_raw: 60,
        squelch_raw: 20,
        cursor_index: 0,
        cursor_editing: false,
    }
}

#[test]
fn test_main_screen_rx() {
    let mut fb = Framebuffer::new();
    let state = rx_state();
    main_screen::render(&mut fb, &state, 0);
    save_png(&fb, "screen-main.png");
}

#[test]
fn test_main_screen_tx() {
    let mut fb = Framebuffer::new();
    let state = RadioState {
        band: 5,
        mode: Mode::Tx,
        transmit_mode: TransmitMode::Cw,
        agc_mode: IfGainMode::AgcSlow,
        rf_gain_mode: RfGainMode::Normal,
        nb_enabled: false,
        clarifier_mode: 0,
        clarifier_raw: 0,
        filter_bw_hz: 500,
        rf_power_centipercent: 7000,
        frequency: 14_060_000,
        rssi_dbm: -53,
        forward_power_mw: 5000,
        vswr_x100: 150,
        volume_raw: 50,
        squelch_raw: 0,
        cursor_index: 0,
        cursor_editing: false,
    };
    main_screen::render(&mut fb, &state, 0);
    save_png(&fb, "screen-main-tx.png");
}

#[test]
fn test_meter_screen() {
    let mut fb = Framebuffer::new();
    let state = RadioState {
        band: 3,
        mode: Mode::Rx,
        transmit_mode: TransmitMode::Usb,
        agc_mode: IfGainMode::AgcFast,
        rf_gain_mode: RfGainMode::RfSingle,
        nb_enabled: false,
        clarifier_mode: 0,
        clarifier_raw: 0,
        filter_bw_hz: 2700,
        rf_power_centipercent: 10000,
        frequency: 7_100_000,
        rssi_dbm: -85,
        forward_power_mw: 15000,
        vswr_x100: 150,
        volume_raw: 50,
        squelch_raw: 0,
        cursor_index: 0,
        cursor_editing: false,
    };
    meter_screen::render(&mut fb, &state, -78);
    save_png(&fb, "screen-meter.png");
}

#[test]
fn test_spectrum_screen() {
    let mut fb = Framebuffer::new();
    let mut buf = WaterfallDisplayBuffer::new();

    for row in 0..90 {
        let mut bins = [-110i8; WATERFALL_BINS];
        for (i, bin) in bins.iter_mut().enumerate() {
            let noise = ((i as i32 * 7 + row as i32 * 13) % 11 - 5) as i8;
            *bin = -105i8.saturating_add(noise);
        }

        let center_peak = |bins: &mut [i8; WATERFALL_BINS], pos: usize, strength: i8| {
            for dx in 0..15 {
                let falloff = (dx as i8) * 3;
                let val = strength.saturating_sub(falloff);
                if pos + dx < WATERFALL_BINS {
                    bins[pos + dx] = bins[pos + dx].max(val);
                }
                if dx > 0 && pos >= dx {
                    bins[pos - dx] = bins[pos - dx].max(val);
                }
            }
        };
        center_peak(&mut bins, 120, -40);
        center_peak(&mut bins, 160, -55);
        center_peak(&mut bins, 80, -65);

        let line = WaterfallLineData {
            center_freq: 7_100_000,
            span_hz: 100_000,
            sweep_status: SweepStatus::Sweeping,
            live_start: 70,
            live_end: 170,
            bins,
        };
        buf.push(&line);
    }

    spectrum_screen::render(&mut fb, &buf, false);
    save_png(&fb, "screen-spectrum.png");
}

#[test]
fn test_menu_screen() {
    let mut fb = Framebuffer::new();
    let mut items = heapless::Vec::<MenuItemView, 16>::new();
    let _ = items.push(MenuItemView {
        label: "Radio Info",
        value: heapless::String::new(),
        is_submenu: true,
    });
    let _ = items.push(MenuItemView {
        label: "Hardware",
        value: heapless::String::new(),
        is_submenu: true,
    });
    let screen = MenuScreen {
        title: "Menu",
        items,
        cursor: 0,
        active: true,
    };
    menu_screen::render(&mut fb, &screen);
    save_png(&fb, "screen-menu.png");
}

#[test]
fn test_fatal_screen() {
    let mut fb = Framebuffer::new();
    let panic_file = [0u8; 64];
    let cmd = CrashInfoCommand {
        reset_reason: 1,
        pc: 0x0801_2A4C,
        lr: 0x0801_2A10,
        panic_line: 0,
        panic_file,
        uptime_secs: 942,
    };
    let error = FatalError::Crash(cmd);
    error_screen::render_fatal(&mut fb, &error);
    save_png(&fb, "screen-fatal.png");
}

#[test]
fn test_all_screenshots_in_readme() {
    let src_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("screenshots.rs");
    let src = std::fs::read_to_string(&src_path).unwrap();
    let readme_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("README.md");
    let readme = std::fs::read_to_string(&readme_path).unwrap();

    let mut missing = Vec::new();
    for line in src.lines() {
        if let Some(start) = line.find("save_png(") {
            let rest = &line[start..];
            if let Some(q1) = rest.find('"') {
                if let Some(q2) = rest[q1 + 1..].find('"') {
                    let filename = &rest[q1 + 1..q1 + 1 + q2];
                    let reference = format!("docs/{}", filename);
                    if !readme.contains(&reference) {
                        missing.push(filename.to_string());
                    }
                }
            }
        }
    }

    assert!(
        missing.is_empty(),
        "Screenshots not referenced in README.md: {:?}",
        missing
    );
}
