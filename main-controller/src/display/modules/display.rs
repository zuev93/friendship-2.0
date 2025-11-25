use embedded_hal::digital::OutputPin;
use embedded_hal_async::spi::SpiBus;

use super::framebuffer::FrameBuffer;
use common::drivers::ssd1315::{SSD1315Config, SSD1315};

pub const DISPLAY_WIDTH: u16 = 128;
pub const DISPLAY_HEIGHT: u16 = 64;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Color {
    Black = 0,
    White = 1,
}

#[allow(dead_code)]
pub struct Display<SPI: SpiBus + 'static, DC: OutputPin> {
    driver: SSD1315<DC>,
    spi: &'static mut SPI,
    front_buffer: FrameBuffer,
    back_buffer: FrameBuffer,
    dirty: bool,
}

#[allow(dead_code)]
impl<SPI: SpiBus, DC: OutputPin> Display<SPI, DC> {
    pub fn new(spi: &'static mut SPI, dc_pin: DC) -> Self {
        let config = SSD1315Config::default();
        let driver = SSD1315::new(dc_pin, config);

        Self {
            driver,
            spi,
            front_buffer: FrameBuffer::new(DISPLAY_WIDTH, DISPLAY_HEIGHT),
            back_buffer: FrameBuffer::new(DISPLAY_WIDTH, DISPLAY_HEIGHT),
            dirty: true,
        }
    }

    pub async fn init(&mut self) -> Result<(), &'static str> {
        self.driver
            .init(self.spi)
            .await
            .map_err(|_| "Failed to initialize display")?;
        self.back_buffer.clear(Color::Black);
        self.dirty = true;
        self.flush().await?;
        Ok(())
    }

    pub async fn clear(&mut self) -> Result<(), SPI::Error> {
        self.back_buffer.clear(Color::Black);
        self.dirty = true;
        self.driver.clear(self.spi).await
    }

    pub async fn flush(&mut self) -> Result<(), &'static str> {
        if self.dirty {
            core::mem::swap(&mut self.front_buffer, &mut self.back_buffer);

            self.driver
                .draw(self.spi, self.front_buffer.buffer())
                .await
                .map_err(|_| "Failed to flush display")?;

            self.back_buffer = self.front_buffer.clone();

            self.dirty = false;
        }
        Ok(())
    }

    pub async fn set_contrast(&mut self, contrast: u8) -> Result<(), SPI::Error> {
        self.driver.set_contrast(self.spi, contrast).await
    }

    pub async fn display_on(&mut self) -> Result<(), SPI::Error> {
        self.driver.display_on(self.spi).await
    }

    pub async fn display_off(&mut self) -> Result<(), SPI::Error> {
        self.driver.display_off(self.spi).await
    }

    pub fn back_buffer(&mut self) -> &mut FrameBuffer {
        self.dirty = true;
        &mut self.back_buffer
    }
}
