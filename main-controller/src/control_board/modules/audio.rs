use embassy_stm32::i2s::{ClockPolarity, Config, Format, Mode as I2sMode, Standard, I2S};
use embassy_stm32::spi::{self, CkPin, MosiPin, TxDma, WsPin};
use embassy_stm32::time::Hertz;
use embassy_stm32::Peri;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use static_cell::StaticCell;

use crate::app::types::Mode;
use crate::consts::AUDIO_BUFFER_SIZE;

static TX_BUFFER: StaticCell<[u16; AUDIO_BUFFER_SIZE]> = StaticCell::new();
static AUDIO_I2S: StaticCell<Mutex<ThreadModeRawMutex, I2S<'static, u16>>> = StaticCell::new();

pub struct Audio {
    i2s: &'static Mutex<ThreadModeRawMutex, I2S<'static, u16>>,
}

impl Audio {
    pub fn new<T: spi::Instance>(
        spi_peri: Peri<'static, T>,
        txsd: Peri<'static, impl MosiPin<T>>,
        ws: Peri<'static, impl WsPin<T>>,
        ck: Peri<'static, impl CkPin<T>>,
        txdma: Peri<'static, impl TxDma<T>>,
    ) -> Self {
        let mut config = Config::default();
        config.frequency = Hertz(48_000);
        config.mode = I2sMode::Master;
        config.standard = Standard::Philips;
        config.format = Format::Data16Channel16;
        config.clock_polarity = ClockPolarity::IdleLow;
        config.master_clock = false;

        let tx_buffer = TX_BUFFER.init([0u16; AUDIO_BUFFER_SIZE]);

        let i2s = I2S::new_txonly_nomck(
            spi_peri, txsd, ws, ck, txdma, tx_buffer, config,
        );
        let i2s = AUDIO_I2S.init(Mutex::new(i2s));

        Self { i2s }
    }

    pub async fn set_mode(&self, mode: Mode) -> Result<(), &'static str> {
        match mode {
            Mode::WarmUp => self.init().await,
            Mode::Rx | Mode::Tx | Mode::StandBy => Ok(()),
        }
    }

    pub async fn write(&self, data: &[u16]) -> Result<(), &'static str> {
        let mut i2s = self.i2s.lock().await;
        i2s.write(data).await.map_err(|_| "I2S write failed")
    }

    async fn init(&self) -> Result<(), &'static str> {
        Ok(())
    }
}
