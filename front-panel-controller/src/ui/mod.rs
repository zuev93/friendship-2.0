pub mod display1;
pub mod meter_bar;

use embedded_graphics::pixelcolor::Rgb565;

pub const BLACK: Rgb565 = Rgb565::new(0, 0, 0);
pub const WHITE: Rgb565 = Rgb565::new(31, 63, 31);
pub const DIM_WHITE: Rgb565 = Rgb565::new(20, 40, 20);
pub const GRAY: Rgb565 = Rgb565::new(12, 24, 12);
pub const DARK_GRAY: Rgb565 = Rgb565::new(6, 12, 6);
pub const GREEN: Rgb565 = Rgb565::new(0, 50, 0);
pub const BRIGHT_GREEN: Rgb565 = Rgb565::new(0, 63, 0);
pub const YELLOW: Rgb565 = Rgb565::new(31, 63, 0);
pub const RED: Rgb565 = Rgb565::new(31, 0, 0);
pub const BLUE: Rgb565 = Rgb565::new(4, 16, 31);
pub const ORANGE: Rgb565 = Rgb565::new(31, 32, 0);
