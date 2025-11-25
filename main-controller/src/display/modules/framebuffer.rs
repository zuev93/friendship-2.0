/*
 * Frame Buffer
 *
 * In-memory representation of the display for double buffering.
 * Allows drawing operations without directly touching the display hardware.
 *
 * For SSD1315 128x64 monochrome display:
 * - 1024 bytes (128 * 64 / 8 bits)
 * - Organized in pages (8 rows per page, 8 pages total)
 */

use super::display::Color;

pub const BUFFER_SIZE: usize = 128 * 64 / 8;

#[allow(dead_code)]
#[derive(Clone)]
pub struct FrameBuffer {
    width: u16,
    height: u16,

    buffer: [u8; BUFFER_SIZE],
}

#[allow(dead_code)]
impl FrameBuffer {
    pub const fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            buffer: [0u8; BUFFER_SIZE],
        }
    }

    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }

    pub fn clear(&mut self, color: Color) {
        let fill_byte = match color {
            Color::White => 0xFF,
            Color::Black => 0x00,
        };
        self.buffer.fill(fill_byte);
    }

    pub fn draw_pixel(&mut self, x: u16, y: u16, color: Color) {
        if x >= self.width || y >= self.height {
            return; // Out of bounds
        }

        let page = y / 8;
        let bit = y % 8;
        let index = (page * self.width + x) as usize;

        match color {
            Color::White => self.buffer[index] |= 1 << bit,
            Color::Black => self.buffer[index] &= !(1 << bit),
        }
    }

    pub fn draw_hline(&mut self, x0: u16, x1: u16, y: u16, color: Color) {
        let x_start = x0.min(x1);
        let x_end = x0.max(x1);
        for x in x_start..=x_end {
            self.draw_pixel(x, y, color);
        }
    }

    pub fn draw_vline(&mut self, x: u16, y0: u16, y1: u16, color: Color) {
        let y_start = y0.min(y1);
        let y_end = y0.max(y1);
        for y in y_start..=y_end {
            self.draw_pixel(x, y, color);
        }
    }

    pub fn draw_line(&mut self, x0: u16, y0: u16, x1: u16, y1: u16, color: Color) {
        let dx = if x1 > x0 { x1 - x0 } else { x0 - x1 } as i16;
        let dy = if y1 > y0 { y1 - y0 } else { y0 - y1 } as i16;

        let sx: i16 = if x0 < x1 { 1 } else { -1 };
        let sy: i16 = if y0 < y1 { 1 } else { -1 };

        let mut err = (if dx > dy { dx } else { -dy }) / 2;
        let mut x = x0 as i16;
        let mut y = y0 as i16;

        loop {
            self.draw_pixel(x as u16, y as u16, color);

            if x == x1 as i16 && y == y1 as i16 {
                break;
            }

            let e2 = err;
            if e2 > -dx {
                err -= dy;
                x += sx;
            }
            if e2 < dy {
                err += dx;
                y += sy;
            }
        }
    }

    pub fn draw_rect(&mut self, x: u16, y: u16, w: u16, h: u16, color: Color) {
        if w == 0 || h == 0 {
            return;
        }
        self.draw_hline(x, x + w - 1, y, color); // Top
        self.draw_hline(x, x + w - 1, y + h - 1, color); // Bottom
        self.draw_vline(x, y, y + h - 1, color); // Left
        self.draw_vline(x + w - 1, y, y + h - 1, color); // Right
    }

    pub fn fill_rect(&mut self, x: u16, y: u16, w: u16, h: u16, color: Color) {
        for dy in 0..h {
            for dx in 0..w {
                self.draw_pixel(x + dx, y + dy, color);
            }
        }
    }

    pub fn draw_text(&mut self, x: u16, y: u16, text: &str, color: Color) {
        let mut cursor_x = x;
        for ch in text.chars() {
            self.draw_char(cursor_x, y, ch, color);
            cursor_x += 6; // 5 pixels + 1 space
        }
    }

    fn draw_char(&mut self, x: u16, y: u16, ch: char, color: Color) {
        let glyph = get_font_glyph(ch);
        for (dy, row) in glyph.iter().enumerate() {
            for dx in 0..5 {
                if (row & (1 << dx)) != 0 {
                    self.draw_pixel(x + dx as u16, y + dy as u16, color);
                }
            }
        }
    }
}

#[allow(dead_code)]
fn get_font_glyph(ch: char) -> [u8; 7] {
    match ch {
        ' ' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        '0' => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
        '1' => [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E],
        '2' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F],
        '3' => [0x0E, 0x11, 0x01, 0x06, 0x01, 0x11, 0x0E],
        '4' => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
        '5' => [0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E],
        '6' => [0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E],
        '7' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        '8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
        '9' => [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x0C],
        'A' | 'a' => [0x0E, 0x11, 0x11, 0x11, 0x1F, 0x11, 0x11],
        'B' | 'b' => [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E],
        'C' | 'c' => [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E],
        'D' | 'd' => [0x1E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1E],
        'E' | 'e' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F],
        'F' | 'f' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10],
        'G' | 'g' => [0x0E, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0E],
        'H' | 'h' => [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'I' | 'i' => [0x0E, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0E],
        'M' | 'm' => [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11],
        'N' | 'n' => [0x11, 0x11, 0x19, 0x15, 0x13, 0x11, 0x11],
        'O' | 'o' => [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'P' | 'p' => [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10],
        'R' | 'r' => [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11],
        'S' | 's' => [0x0E, 0x11, 0x10, 0x0E, 0x01, 0x11, 0x0E],
        'T' | 't' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        'U' | 'u' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'W' | 'w' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x1B, 0x11],
        'X' | 'x' => [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11],
        '-' => [0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00],
        '.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x0C],
        ':' => [0x00, 0x0C, 0x0C, 0x00, 0x0C, 0x0C, 0x00],
        '/' => [0x01, 0x01, 0x02, 0x04, 0x08, 0x10, 0x10],
        _ => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00], // Unknown char
    }
}
