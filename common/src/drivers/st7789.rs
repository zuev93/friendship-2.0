use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use embassy_time::Timer;
use embedded_hal::digital::OutputPin;
use embedded_hal_async::spi::SpiBus;

pub const DISPLAY_WIDTH: u16 = 240;
pub const DISPLAY_HEIGHT: u16 = 135;

const CMD_SWRESET: u8 = 0x01;
const CMD_SLPOUT: u8 = 0x11;
const CMD_NORON: u8 = 0x13;
const CMD_INVON: u8 = 0x21;
const CMD_DISPON: u8 = 0x29;
const CMD_CASET: u8 = 0x2A;
const CMD_RASET: u8 = 0x2B;
const CMD_RAMWR: u8 = 0x2C;
const CMD_MADCTL: u8 = 0x36;
const CMD_COLMOD: u8 = 0x3A;
const CMD_PORCTRL: u8 = 0xB2;
const CMD_GCTRL: u8 = 0xB7;
const CMD_VCOMS: u8 = 0xBB;
const CMD_LCMCTRL: u8 = 0xC0;
const CMD_VDVVRHEN: u8 = 0xC2;
const CMD_VRHS: u8 = 0xC3;
const CMD_VDVSET: u8 = 0xC4;
const CMD_FRCTR2: u8 = 0xC6;
const CMD_PWCTRL1: u8 = 0xD0;
const CMD_PVGAMCTRL: u8 = 0xE0;
const CMD_NVGAMCTRL: u8 = 0xE1;

const MADCTL_MX: u8 = 0x40;
const MADCTL_MY: u8 = 0x80;
const MADCTL_MV: u8 = 0x20;
const MADCTL_RGB: u8 = 0x00;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Rotation {
    Portrait0,
    Landscape90,
    Portrait180,
    Landscape270,
}

impl Rotation {
    fn madctl(self) -> u8 {
        match self {
            Rotation::Portrait0 => MADCTL_MX | MADCTL_MY | MADCTL_RGB,
            Rotation::Landscape90 => MADCTL_MV | MADCTL_MY | MADCTL_RGB,
            Rotation::Portrait180 => MADCTL_RGB,
            Rotation::Landscape270 => MADCTL_MV | MADCTL_MX | MADCTL_RGB,
        }
    }

    fn offsets(self) -> (u16, u16) {
        match self {
            Rotation::Portrait0 => (52, 40),
            Rotation::Landscape90 => (40, 53),
            Rotation::Portrait180 => (53, 40),
            Rotation::Landscape270 => (40, 52),
        }
    }

    pub fn width(self) -> u16 {
        match self {
            Rotation::Portrait0 | Rotation::Portrait180 => DISPLAY_HEIGHT,
            Rotation::Landscape90 | Rotation::Landscape270 => DISPLAY_WIDTH,
        }
    }

    pub fn height(self) -> u16 {
        match self {
            Rotation::Portrait0 | Rotation::Portrait180 => DISPLAY_WIDTH,
            Rotation::Landscape90 | Rotation::Landscape270 => DISPLAY_HEIGHT,
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Spi,
    Gpio,
}

pub struct ST7789<SPI, DC, RST>
where
    SPI: SpiBus + 'static,
    DC: OutputPin + 'static,
    RST: OutputPin + 'static,
{
    spi: &'static Mutex<ThreadModeRawMutex, SPI>,
    dc: DC,
    rst: RST,
    rotation: Rotation,
    col_offset: u16,
    row_offset: u16,
}

impl<SPI, DC, RST> ST7789<SPI, DC, RST>
where
    SPI: SpiBus + 'static,
    DC: OutputPin + 'static,
    RST: OutputPin + 'static,
{
    pub fn new(
        spi: &'static Mutex<ThreadModeRawMutex, SPI>,
        dc: DC,
        rst: RST,
        rotation: Rotation,
    ) -> Self {
        let (col_offset, row_offset) = rotation.offsets();
        Self {
            spi,
            dc,
            rst,
            rotation,
            col_offset,
            row_offset,
        }
    }

    pub async fn init(&mut self) -> Result<(), Error> {
        self.hard_reset().await?;

        self.send_command(CMD_SWRESET).await?;
        Timer::after_millis(150).await;

        self.send_command(CMD_SLPOUT).await?;
        Timer::after_millis(150).await;

        self.send_command(CMD_COLMOD).await?;
        self.send_data(&[0x55]).await?;

        self.send_command(CMD_MADCTL).await?;
        self.send_data(&[self.rotation.madctl()]).await?;

        self.send_command(CMD_PORCTRL).await?;
        self.send_data(&[0x0C, 0x0C, 0x00, 0x33, 0x33]).await?;

        self.send_command(CMD_GCTRL).await?;
        self.send_data(&[0x35]).await?;

        self.send_command(CMD_VCOMS).await?;
        self.send_data(&[0x28]).await?;

        self.send_command(CMD_LCMCTRL).await?;
        self.send_data(&[0x0C]).await?;

        self.send_command(CMD_VDVVRHEN).await?;
        self.send_data(&[0x01]).await?;

        self.send_command(CMD_VRHS).await?;
        self.send_data(&[0x10]).await?;

        self.send_command(CMD_VDVSET).await?;
        self.send_data(&[0x20]).await?;

        self.send_command(CMD_FRCTR2).await?;
        self.send_data(&[0x0F]).await?;

        self.send_command(CMD_PWCTRL1).await?;
        self.send_data(&[0xA4, 0xA1]).await?;

        self.send_command(CMD_PVGAMCTRL).await?;
        self.send_data(&[
            0xD0, 0x00, 0x02, 0x07, 0x0A, 0x28, 0x32, 0x44, 0x42, 0x06, 0x0E, 0x12, 0x14, 0x17,
        ])
        .await?;

        self.send_command(CMD_NVGAMCTRL).await?;
        self.send_data(&[
            0xD0, 0x00, 0x02, 0x07, 0x0A, 0x28, 0x31, 0x54, 0x47, 0x0E, 0x1C, 0x17, 0x1B, 0x1E,
        ])
        .await?;

        self.send_command(CMD_INVON).await?;
        self.send_command(CMD_NORON).await?;
        self.send_command(CMD_DISPON).await?;
        Timer::after_millis(120).await;

        Ok(())
    }

    pub async fn set_window(
        &mut self,
        x: u16,
        y: u16,
        w: u16,
        h: u16,
    ) -> Result<(), Error> {
        let x0 = x + self.col_offset;
        let x1 = x + w - 1 + self.col_offset;
        let y0 = y + self.row_offset;
        let y1 = y + h - 1 + self.row_offset;

        self.send_command(CMD_CASET).await?;
        self.send_data(&[(x0 >> 8) as u8, x0 as u8, (x1 >> 8) as u8, x1 as u8])
            .await?;

        self.send_command(CMD_RASET).await?;
        self.send_data(&[(y0 >> 8) as u8, y0 as u8, (y1 >> 8) as u8, y1 as u8])
            .await?;

        self.send_command(CMD_RAMWR).await?;
        Ok(())
    }

    pub async fn write_pixels(&mut self, data: &[u8]) -> Result<(), Error> {
        self.send_data(data).await
    }

    pub async fn draw(&mut self, framebuffer: &[u8]) -> Result<(), Error> {
        let w = self.rotation.width();
        let h = self.rotation.height();
        self.set_window(0, 0, w, h).await?;
        self.send_data(framebuffer).await
    }

    pub async fn fill_color(&mut self, color: u16) -> Result<(), Error> {
        let w = self.rotation.width();
        let h = self.rotation.height();
        self.set_window(0, 0, w, h).await?;

        let hi = (color >> 8) as u8;
        let lo = color as u8;
        const CHUNK_PIXELS: usize = 240;
        let mut chunk = [0u8; CHUNK_PIXELS * 2];
        for i in 0..CHUNK_PIXELS {
            chunk[i * 2] = hi;
            chunk[i * 2 + 1] = lo;
        }

        let total_pixels = w as u32 * h as u32;
        let mut remaining = total_pixels;
        while remaining > 0 {
            let count = remaining.min(CHUNK_PIXELS as u32);
            self.send_data(&chunk[..count as usize * 2]).await?;
            remaining -= count;
        }

        Ok(())
    }

    pub async fn set_rotation(&mut self, rotation: Rotation) -> Result<(), Error> {
        self.rotation = rotation;
        let (col_offset, row_offset) = rotation.offsets();
        self.col_offset = col_offset;
        self.row_offset = row_offset;

        self.send_command(CMD_MADCTL).await?;
        self.send_data(&[rotation.madctl()]).await
    }

    async fn hard_reset(&mut self) -> Result<(), Error> {
        self.rst.set_high().map_err(|_| Error::Gpio)?;
        Timer::after_millis(10).await;
        self.rst.set_low().map_err(|_| Error::Gpio)?;
        Timer::after_millis(10).await;
        self.rst.set_high().map_err(|_| Error::Gpio)?;
        Timer::after_millis(120).await;
        Ok(())
    }

    async fn send_command(&mut self, cmd: u8) -> Result<(), Error> {
        self.dc.set_low().map_err(|_| Error::Gpio)?;
        self.spi
            .lock()
            .await
            .write(&[cmd])
            .await
            .map_err(|_| Error::Spi)
    }

    async fn send_data(&mut self, data: &[u8]) -> Result<(), Error> {
        self.dc.set_high().map_err(|_| Error::Gpio)?;
        self.spi
            .lock()
            .await
            .write(data)
            .await
            .map_err(|_| Error::Spi)
    }
}
