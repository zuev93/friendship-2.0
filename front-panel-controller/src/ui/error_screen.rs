use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::Text,
};

use druzhba_common::error::BsodError;
use druzhba_common::protocol_types::CrashInfoCommand;

use crate::state::input::FatalError;
use crate::ui::{BLACK, RED, WHITE};

pub fn render_fatal(target: &mut impl DrawTarget<Color = Rgb565>, error: &FatalError) {
    let _ = Rectangle::new(Point::zero(), Size::new(240, 135))
        .into_styled(PrimitiveStyle::with_fill(RED))
        .draw(target);

    let title_style = MonoTextStyle::new(&FONT_6X10, WHITE);
    let detail_style = MonoTextStyle::new(&FONT_6X10, BLACK);

    match error {
        FatalError::Init(bsod) => {
            Text::new("FATAL ERROR", Point::new(72, 50), title_style)
                .draw(target)
                .ok();

            let msg = match bsod {
                BsodError::FrontPanelInitFailed => "Front panel init failed",
                BsodError::DisplayInitFailed => "Display init failed",
                BsodError::MainBoardInitFailed => "Main board init failed",
                BsodError::Crash => "Main board crashed",
            };

            Text::new(msg, Point::new(30, 75), detail_style)
                .draw(target)
                .ok();
        }
        FatalError::Crash(cmd) => {
            render_crash_detail(target, cmd, title_style, detail_style);
        }
    }
}

fn render_crash_detail(
    target: &mut impl DrawTarget<Color = Rgb565>,
    cmd: &CrashInfoCommand,
    title_style: MonoTextStyle<'_, Rgb565>,
    detail_style: MonoTextStyle<'_, Rgb565>,
) {
    let title = match cmd.reset_reason {
        1 => "CRASH: HardFault",
        2 => "CRASH: Panic",
        3 => "CRASH: Watchdog",
        _ => "CRASH: Unknown",
    };
    Text::new(title, Point::new(60, 20), title_style)
        .draw(target)
        .ok();

    let mut line2 = [0u8; 40];
    let line2_len = match cmd.reset_reason {
        1 => fmt_hex_line(&mut line2, b"PC:", cmd.pc, b" LR:", cmd.lr),
        2 => {
            let file_len = cmd.panic_file.iter().position(|&b| b == 0).unwrap_or(64);
            let file = &cmd.panic_file[..file_len.min(28)];
            let mut pos = 0;
            line2[pos..pos + file.len()].copy_from_slice(file);
            pos += file.len();
            line2[pos] = b':';
            pos += 1;
            pos += fmt_u32_decimal(&mut line2[pos..], cmd.panic_line);
            pos
        }
        _ => 0,
    };

    if line2_len > 0 {
        if let Ok(s) = core::str::from_utf8(&line2[..line2_len]) {
            Text::new(s, Point::new(10, 55), detail_style)
                .draw(target)
                .ok();
        }
    }

    let mut line3 = [0u8; 20];
    let l3_start = b"Uptime: ";
    line3[..l3_start.len()].copy_from_slice(l3_start);
    let mut pos = l3_start.len();
    pos += fmt_u32_decimal(&mut line3[pos..], cmd.uptime_secs);
    line3[pos] = b's';
    pos += 1;
    if let Ok(s) = core::str::from_utf8(&line3[..pos]) {
        Text::new(s, Point::new(10, 90), detail_style)
            .draw(target)
            .ok();
    }
}

fn fmt_hex_line(buf: &mut [u8], prefix1: &[u8], val1: u32, prefix2: &[u8], val2: u32) -> usize {
    let mut pos = 0;
    buf[pos..pos + prefix1.len()].copy_from_slice(prefix1);
    pos += prefix1.len();
    pos += fmt_hex32(&mut buf[pos..], val1);
    buf[pos..pos + prefix2.len()].copy_from_slice(prefix2);
    pos += prefix2.len();
    pos += fmt_hex32(&mut buf[pos..], val2);
    pos
}

fn fmt_hex32(buf: &mut [u8], val: u32) -> usize {
    buf[0] = b'0';
    buf[1] = b'x';
    let mut i = 0;
    while i < 8 {
        let nibble = ((val >> (28 - i * 4)) & 0xF) as u8;
        buf[2 + i] = if nibble < 10 {
            b'0' + nibble
        } else {
            b'A' + nibble - 10
        };
        i += 1;
    }
    10
}

fn fmt_u32_decimal(buf: &mut [u8], mut val: u32) -> usize {
    if val == 0 {
        buf[0] = b'0';
        return 1;
    }
    let mut tmp = [0u8; 10];
    let mut len = 0;
    while val > 0 {
        tmp[len] = b'0' + (val % 10) as u8;
        val /= 10;
        len += 1;
    }
    let mut i = 0;
    while i < len {
        buf[i] = tmp[len - 1 - i];
        i += 1;
    }
    len
}
