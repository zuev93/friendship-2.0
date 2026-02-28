use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
};

pub fn draw_meter_bar(
    target: &mut impl DrawTarget<Color = Rgb565>,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    value: u16,
    max: u16,
    bar_color: Rgb565,
    bg_color: Rgb565,
) {
    let fill_w = if max > 0 {
        ((value as u32).min(max as u32) * width) / max as u32
    } else {
        0
    };

    if fill_w > 0 {
        let _ = Rectangle::new(Point::new(x, y), Size::new(fill_w, height))
            .into_styled(PrimitiveStyle::with_fill(bar_color))
            .draw(target);
    }

    let remaining = width - fill_w;
    if remaining > 0 {
        let _ = Rectangle::new(
            Point::new(x + fill_w as i32, y),
            Size::new(remaining, height),
        )
        .into_styled(PrimitiveStyle::with_fill(bg_color))
        .draw(target);
    }
}

pub fn draw_gradient_meter(
    target: &mut impl DrawTarget<Color = Rgb565>,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    value: u16,
    max: u16,
    threshold: u16,
    color_low: Rgb565,
    color_high: Rgb565,
    bg_color: Rgb565,
) {
    let fill_w = if max > 0 {
        ((value as u32).min(max as u32) * width) / max as u32
    } else {
        0
    };

    let threshold_w = ((threshold as u32).min(max as u32) * width) / max as u32;

    let low_w = fill_w.min(threshold_w);
    if low_w > 0 {
        let _ = Rectangle::new(Point::new(x, y), Size::new(low_w, height))
            .into_styled(PrimitiveStyle::with_fill(color_low))
            .draw(target);
    }

    if fill_w > threshold_w {
        let high_w = fill_w - threshold_w;
        let _ = Rectangle::new(
            Point::new(x + threshold_w as i32, y),
            Size::new(high_w, height),
        )
        .into_styled(PrimitiveStyle::with_fill(color_high))
        .draw(target);
    }

    let remaining = width - fill_w;
    if remaining > 0 {
        let _ = Rectangle::new(
            Point::new(x + fill_w as i32, y),
            Size::new(remaining, height),
        )
        .into_styled(PrimitiveStyle::with_fill(bg_color))
        .draw(target);
    }
}
