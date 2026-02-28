use core::convert::Infallible;
use embedded_graphics_core::{
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Size},
    pixelcolor::{raw::{RawData, RawU16}, Rgb565},
    Pixel,
};

pub const WIDTH: usize = 240;
pub const HEIGHT: usize = 135;
pub const BUFFER_SIZE: usize = WIDTH * HEIGHT * 2;

pub struct BufferedDisplay {
    buffers: [[u8; BUFFER_SIZE]; 2],
    back: u8,
}

impl BufferedDisplay {
    pub fn new() -> Self {
        Self {
            buffers: [[0u8; BUFFER_SIZE]; 2],
            back: 0,
        }
    }

    pub fn swap(&mut self) -> &[u8; BUFFER_SIZE] {
        let front = self.back as usize;
        self.back = 1 - self.back;
        &self.buffers[front]
    }

    fn back_buffer_mut(&mut self) -> &mut [u8; BUFFER_SIZE] {
        &mut self.buffers[self.back as usize]
    }
}

impl OriginDimensions for BufferedDisplay {
    fn size(&self) -> Size {
        Size::new(WIDTH as u32, HEIGHT as u32)
    }
}

impl DrawTarget for BufferedDisplay {
    type Color = Rgb565;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        let buf = self.back_buffer_mut();
        for Pixel(point, color) in pixels {
            let x = point.x;
            let y = point.y;
            if x >= 0 && x < WIDTH as i32 && y >= 0 && y < HEIGHT as i32 {
                let index = (y as usize * WIDTH + x as usize) * 2;
                let raw = RawU16::from(color).into_inner();
                buf[index] = (raw >> 8) as u8;
                buf[index + 1] = raw as u8;
            }
        }
        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        let raw = RawU16::from(color).into_inner();
        let hi = (raw >> 8) as u8;
        let lo = raw as u8;
        let buf = self.back_buffer_mut();
        let mut i = 0;
        while i < BUFFER_SIZE {
            buf[i] = hi;
            buf[i + 1] = lo;
            i += 2;
        }
        Ok(())
    }
}
