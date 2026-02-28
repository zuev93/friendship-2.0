use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::Text,
};

use druzhba_common::error::BsodError;

use crate::ui::{BLACK, RED, WHITE};

pub fn render_bsod(target: &mut impl DrawTarget<Color = Rgb565>, error: BsodError) {
    let _ = Rectangle::new(Point::zero(), Size::new(240, 135))
        .into_styled(PrimitiveStyle::with_fill(RED))
        .draw(target);

    let title_style = MonoTextStyle::new(&FONT_6X10, WHITE);
    Text::new("FATAL ERROR", Point::new(72, 50), title_style)
        .draw(target)
        .ok();

    let msg = match error {
        BsodError::FrontPanelInitFailed => "Front panel init failed",
        BsodError::DisplayInitFailed => "Display init failed",
        BsodError::MainBoardInitFailed => "Main board init failed",
    };

    let msg_style = MonoTextStyle::new(&FONT_6X10, BLACK);
    Text::new(msg, Point::new(30, 75), msg_style).draw(target).ok();
}

pub fn render_error(target: &mut impl DrawTarget<Color = Rgb565>, message: &str) {
    let _ = Rectangle::new(Point::zero(), Size::new(240, 135))
        .into_styled(PrimitiveStyle::with_fill(BLACK))
        .draw(target);

    let label_style = MonoTextStyle::new(&FONT_6X10, RED);
    Text::new("ERROR", Point::new(90, 50), label_style)
        .draw(target)
        .ok();

    let msg_style = MonoTextStyle::new(&FONT_6X10, WHITE);
    Text::new(message, Point::new(10, 75), msg_style)
        .draw(target)
        .ok();
}
