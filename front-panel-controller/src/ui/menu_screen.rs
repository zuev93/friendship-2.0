use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::Text,
};

use crate::state::menu::MenuScreen;
use crate::ui::{BLACK, BLUE, DARK_GRAY, DIM_WHITE, GRAY, WHITE};

const TITLE_HEIGHT: i32 = 14;
const ITEM_HEIGHT: i32 = 14;
const DISPLAY_WIDTH: u32 = 240;
const DISPLAY_HEIGHT: u32 = 135;

pub fn render(target: &mut impl DrawTarget<Color = Rgb565>, screen: &MenuScreen) {
    let _ = Rectangle::new(Point::zero(), Size::new(DISPLAY_WIDTH, DISPLAY_HEIGHT))
        .into_styled(PrimitiveStyle::with_fill(BLACK))
        .draw(target);

    let _ = Rectangle::new(
        Point::new(0, 0),
        Size::new(DISPLAY_WIDTH, TITLE_HEIGHT as u32),
    )
    .into_styled(PrimitiveStyle::with_fill(DARK_GRAY))
    .draw(target);

    let title_style = MonoTextStyle::new(&FONT_6X10, WHITE);
    Text::new(screen.title, Point::new(2, 10), title_style)
        .draw(target)
        .ok();

    let normal_style = MonoTextStyle::new(&FONT_6X10, DIM_WHITE);
    let selected_style = MonoTextStyle::new(&FONT_6X10, WHITE);
    let value_style = MonoTextStyle::new(&FONT_6X10, GRAY);

    let max_visible = ((DISPLAY_HEIGHT as i32 - TITLE_HEIGHT) / ITEM_HEIGHT) as u8;
    let scroll_offset = if screen.cursor >= max_visible {
        screen.cursor - max_visible + 1
    } else {
        0
    };

    for (i, item) in screen.items.iter().enumerate() {
        let idx = i as u8;
        if idx < scroll_offset {
            continue;
        }
        let row = (idx - scroll_offset) as i32;
        if row >= max_visible as i32 {
            break;
        }

        let y = TITLE_HEIGHT + row * ITEM_HEIGHT;

        if idx == screen.cursor {
            let _ = Rectangle::new(
                Point::new(0, y),
                Size::new(DISPLAY_WIDTH, ITEM_HEIGHT as u32),
            )
            .into_styled(PrimitiveStyle::with_fill(BLUE))
            .draw(target);
        }

        let style = if idx == screen.cursor {
            selected_style
        } else {
            normal_style
        };

        Text::new(item.label, Point::new(4, y + 10), style)
            .draw(target)
            .ok();

        if item.is_submenu {
            Text::new(">", Point::new(232, y + 10), style)
                .draw(target)
                .ok();
        } else if !item.value.is_empty() {
            let val_width = item.value.len() as i32 * 6;
            Text::new(
                &item.value,
                Point::new(DISPLAY_WIDTH as i32 - val_width - 4, y + 10),
                value_style,
            )
            .draw(target)
            .ok();
        }
    }
}
