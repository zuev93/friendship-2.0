use embassy_sync::mutex::Mutex;
use crate::PlatformMutex;
use embedded_hal::digital::OutputPin;
use embedded_hal_async::spi::SpiBus;

#[derive(Debug)]
pub enum Error {
    Spi,
    Gpio,
}

#[allow(dead_code)]
pub const DISPLAY_WIDTH: u8 = 128;
pub const DISPLAY_HEIGHT: u8 = 64;
pub const DISPLAY_PAGES: u8 = DISPLAY_HEIGHT / 8;

const CMD_SET_CONTRAST: u8 = 0x81;
const CMD_DISPLAY_ALL_ON_RESUME: u8 = 0xA4;
const CMD_NORMAL_DISPLAY: u8 = 0xA6;
const CMD_DISPLAY_OFF: u8 = 0xAE;
const CMD_DISPLAY_ON: u8 = 0xAF;

const CMD_DEACTIVATE_SCROLL: u8 = 0x2E;

const CMD_SET_MEMORY_MODE: u8 = 0x20;
const CMD_SET_COLUMN_ADDR: u8 = 0x21;
const CMD_SET_PAGE_ADDR: u8 = 0x22;

const CMD_SET_START_LINE: u8 = 0x40;
const CMD_SET_SEGMENT_REMAP: u8 = 0xA1;
const CMD_SET_MULTIPLEX: u8 = 0xA8;
const CMD_SET_COM_SCAN_DEC: u8 = 0xC8;
const CMD_SET_DISPLAY_OFFSET: u8 = 0xD3;
const CMD_SET_COM_PINS: u8 = 0xDA;

const CMD_SET_DISPLAY_CLOCK: u8 = 0xD5;
const CMD_SET_PRECHARGE: u8 = 0xD9;
const CMD_SET_VCOMH_DESELECT: u8 = 0xDB;

const CMD_CHARGE_PUMP: u8 = 0x8D;

#[derive(Clone, Copy)]
pub struct SSD1315Config {
    pub width: u8,
    pub height: u8,
    pub external_vcc: bool,
}

impl Default for SSD1315Config {
    fn default() -> Self {
        Self {
            width: DISPLAY_WIDTH,
            height: DISPLAY_HEIGHT,
            external_vcc: false, // Use charge pump by default
        }
    }
}

pub struct SSD1315<SPI, DC, CS>
where
    SPI: SpiBus + 'static,
    DC: OutputPin + 'static,
    CS: OutputPin + 'static,
{
    spi: &'static Mutex<PlatformMutex, SPI>,
    dc: &'static Mutex<PlatformMutex, DC>,
    cs: CS,
    config: SSD1315Config,
}

impl<SPI, DC, CS> SSD1315<SPI, DC, CS>
where
    SPI: SpiBus + 'static,
    DC: OutputPin + 'static,
    CS: OutputPin + 'static,
{
    pub fn new(
        spi: &'static Mutex<PlatformMutex, SPI>,
        dc: &'static Mutex<PlatformMutex, DC>,
        cs: CS,
        config: SSD1315Config,
    ) -> Self {
        Self {
            spi,
            dc,
            cs,
            config,
        }
    }

    pub async fn init(&mut self) -> Result<(), Error> {
        self.send_command(CMD_DISPLAY_OFF).await?;

        self.send_command(CMD_SET_DISPLAY_CLOCK).await?;
        self.send_command(0x80).await?;

        self.send_command(CMD_SET_MULTIPLEX).await?;
        self.send_command(self.config.height - 1).await?;

        self.send_command(CMD_SET_DISPLAY_OFFSET).await?;
        self.send_command(0x00).await?;

        self.send_command(CMD_SET_START_LINE).await?;

        self.send_command(CMD_CHARGE_PUMP).await?;
        if self.config.external_vcc {
            self.send_command(0x10).await?;
        } else {
            self.send_command(0x14).await?;
        }

        self.send_command(CMD_SET_MEMORY_MODE).await?;
        self.send_command(0x00).await?;

        self.send_command(CMD_SET_SEGMENT_REMAP).await?;

        self.send_command(CMD_SET_COM_SCAN_DEC).await?;

        self.send_command(CMD_SET_COM_PINS).await?;
        if self.config.height == 64 {
            self.send_command(0x12).await?;
        } else {
            self.send_command(0x02).await?;
        }

        self.send_command(CMD_SET_CONTRAST).await?;
        if self.config.external_vcc {
            self.send_command(0x9F).await?;
        } else {
            self.send_command(0xCF).await?;
        }

        self.send_command(CMD_SET_PRECHARGE).await?;
        if self.config.external_vcc {
            self.send_command(0x22).await?;
        } else {
            self.send_command(0xF1).await?;
        }

        self.send_command(CMD_SET_VCOMH_DESELECT).await?;
        self.send_command(0x40).await?;

        self.send_command(CMD_DISPLAY_ALL_ON_RESUME).await?;

        self.send_command(CMD_NORMAL_DISPLAY).await?;

        self.send_command(CMD_DEACTIVATE_SCROLL).await?;

        self.send_command(CMD_DISPLAY_ON).await?;

        Ok(())
    }

    async fn send_command(&mut self, cmd: u8) -> Result<(), Error> {
        self.cs.set_low().map_err(|_| Error::Gpio)?;
        {
            let mut spi = self.spi.lock().await;
            let mut dc = self.dc.lock().await;
            dc.set_low().map_err(|_| Error::Gpio)?;
            spi.write(&[cmd]).await.map_err(|_| Error::Spi)?;
        }
        self.cs.set_high().map_err(|_| Error::Gpio)?;
        Ok(())
    }

    async fn send_data(&mut self, data: &[u8]) -> Result<(), Error> {
        self.cs.set_low().map_err(|_| Error::Gpio)?;
        {
            let mut spi = self.spi.lock().await;
            let mut dc = self.dc.lock().await;
            dc.set_high().map_err(|_| Error::Gpio)?;
            spi.write(data).await.map_err(|_| Error::Spi)?;
        }
        self.cs.set_high().map_err(|_| Error::Gpio)?;
        Ok(())
    }

    pub async fn set_contrast(&mut self, contrast: u8) -> Result<(), Error> {
        self.send_command(CMD_SET_CONTRAST).await?;
        self.send_command(contrast).await?;
        Ok(())
    }

    pub async fn display_on(&mut self) -> Result<(), Error> {
        self.send_command(CMD_DISPLAY_ON).await
    }

    pub async fn display_off(&mut self) -> Result<(), Error> {
        self.send_command(CMD_DISPLAY_OFF).await
    }

    pub async fn set_draw_area(
        &mut self,
        x: u8,
        y: u8,
        width: u8,
        height: u8,
    ) -> Result<(), Error> {
        let x_end = (x + width - 1).min(self.config.width - 1);
        let y_page = y / 8;
        let y_page_end = ((y + height - 1) / 8).min(DISPLAY_PAGES - 1);

        self.send_command(CMD_SET_COLUMN_ADDR).await?;
        self.send_command(x).await?;
        self.send_command(x_end).await?;

        self.send_command(CMD_SET_PAGE_ADDR).await?;
        self.send_command(y_page).await?;
        self.send_command(y_page_end).await?;

        Ok(())
    }

    pub async fn draw(&mut self, framebuffer: &[u8]) -> Result<(), Error> {
        self.set_draw_area(0, 0, self.config.width, self.config.height)
            .await?;

        self.send_data(framebuffer).await?;

        Ok(())
    }

    pub async fn clear(&mut self) -> Result<(), Error> {
        self.set_draw_area(0, 0, self.config.width, self.config.height)
            .await?;

        let pages = (self.config.height / 8) as usize;
        let buffer_size = self.config.width as usize * pages;

        const CHUNK_SIZE: usize = 128;
        let clear_chunk = [0u8; CHUNK_SIZE];

        let mut remaining = buffer_size;
        while remaining > 0 {
            let chunk_len = remaining.min(CHUNK_SIZE);
            self.send_data(&clear_chunk[..chunk_len]).await?;
            remaining -= chunk_len;
        }

        Ok(())
    }
}
