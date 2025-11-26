use crate::app::types::Mode;
use crate::consts::AUDIO_BUFFER_SIZE;
use crate::i2c_map;
use crate::main_board::types::MainBoardI2CMutex;
use common::drivers::pca9534::{Pin, PCA9534};
use common::drivers::pcm3060::Pcm3060;
use embassy_stm32::gpio::Speed;

use embassy_stm32::i2s::{
    ClockPolarity, Config, Format, Mode as I2sMode, Reader, Standard, Writer, I2S,
};
use embassy_stm32::spi::{self, CkPin, MckPin, MisoPin, MosiPin, RxDma, TxDma, WsPin};
use embassy_stm32::time::Hertz;
use embassy_stm32::Peri;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use static_cell::StaticCell;

const PCA9534_AUDIO_PANEL_ADDR: u8 = 0x20;
static TX_BUFFER: StaticCell<[u16; AUDIO_BUFFER_SIZE]> = StaticCell::new();
static RX_BUFFER: StaticCell<[u16; AUDIO_BUFFER_SIZE]> = StaticCell::new();
static AUDIO_I2S: StaticCell<Mutex<ThreadModeRawMutex, I2S<'static, u16>>> = StaticCell::new();

pub struct AudioPanel {
    pcm3060: Pcm3060,
    pca9534: PCA9534,
    i2c: &'static MainBoardI2CMutex,
    i2s: &'static Mutex<ThreadModeRawMutex, I2S<'static, u16>>,
}

impl AudioPanel {
    pub fn new<T: spi::Instance>(
        i2c: &'static MainBoardI2CMutex,
        spi_peri: Peri<'static, T>,
        txsd: Peri<'static, impl MosiPin<T>>,
        rxsd: Peri<'static, impl MisoPin<T>>,
        ws: Peri<'static, impl WsPin<T>>,
        ck: Peri<'static, impl CkPin<T>>,
        mck: Peri<'static, impl MckPin<T>>,
        txdma: Peri<'static, impl TxDma<T>>,
        rxdma: Peri<'static, impl RxDma<T>>,
    ) -> Self {
        let pcm3060 = Pcm3060::new(i2c_map::PCM3060_AUDIO_PANEL_ADDR);
        let pca9534 = PCA9534::new(PCA9534_AUDIO_PANEL_ADDR);

        let mut config = Config::default();
        config.frequency = Hertz(48_000);
        config.gpio_speed = Speed::VeryHigh;
        config.mode = I2sMode::Master;
        config.standard = Standard::Philips;
        config.format = Format::Data16Channel16;
        config.clock_polarity = ClockPolarity::IdleLow;
        config.master_clock = false;

        let tx_buffer = TX_BUFFER.init([0u16; AUDIO_BUFFER_SIZE]);
        let rx_buffer = RX_BUFFER.init([0u16; AUDIO_BUFFER_SIZE]);

        let i2s = I2S::new_full_duplex(
            spi_peri, txsd, rxsd, ws, ck, mck, txdma, tx_buffer, rxdma, rx_buffer, config,
        );
        let i2s = AUDIO_I2S.init(Mutex::new(i2s));

        Self {
            pcm3060,
            pca9534,
            i2c,
            i2s,
        }
    }

    pub async fn set_mode(&mut self, mode: Mode) -> Result<(), &'static str> {
        match mode {
            Mode::Rx => self.set_signal_detector_to_adc().await,
            Mode::Tx => self.set_signal_detector_to_dac().await,
            Mode::WarmUp => self.init().await,
            Mode::StandBy => Ok(()),
        }
    }

    pub async fn split_i2s(
        &mut self,
    ) -> (Reader<'static, 'static, u16>, Writer<'static, 'static, u16>) {
        let mut i2s_guard = self.i2s.lock().await;
        let (reader, writer) = unsafe {
            core::mem::transmute::<
                (Reader<'_, '_, u16>, Writer<'_, '_, u16>),
                (Reader<'static, 'static, u16>, Writer<'static, 'static, u16>),
            >(i2s_guard.split().unwrap())
        };
        core::mem::forget(i2s_guard);
        (reader, writer)
    }

    pub async fn init(&mut self) -> Result<(), &'static str> {
        let mut i2c_guard = self.i2c.lock().await;

        self.pca9534
            .init(&mut *i2c_guard)
            .await
            .map_err(|_| "Failed to initialize PCA9534")?;

        self.pca9534
            .set_pin_direction(&mut *i2c_guard, Pin::Pin0, false)
            .await
            .map_err(|_| "Failed to configure P0 as output")?;

        self.pca9534
            .set_pin_direction(&mut *i2c_guard, Pin::Pin1, false)
            .await
            .map_err(|_| "Failed to configure P1 as output")?;

        self.reset_pcm3060().await?;

        self.pcm3060
            .init(&mut *i2c_guard)
            .await
            .map_err(|_| "Failed to initialize PCM3060")?;

        let mut i2s_guard = self.i2s.lock().await;
        i2s_guard.start();

        Ok(())
    }

    async fn reset_pcm3060(&mut self) -> Result<(), &'static str> {
        let mut i2c_guard = self.i2c.lock().await;

        self.pca9534
            .write_pin(&mut *i2c_guard, Pin::Pin0, false)
            .await
            .map_err(|_| "Failed to set PCM3060 reset low")?;

        embassy_time::Timer::after_millis(10).await;

        self.pca9534
            .write_pin(&mut *i2c_guard, Pin::Pin0, true)
            .await
            .map_err(|_| "Failed to set PCM3060 reset high")?;

        Ok(())
    }

    async fn set_signal_detector_to_adc(&mut self) -> Result<(), &'static str> {
        let mut i2c_guard = self.i2c.lock().await;
        self.pca9534
            .write_pin(&mut *i2c_guard, Pin::Pin1, false)
            .await
            .map_err(|_| "Failed to set signal detector to ADC")?;
        Ok(())
    }

    async fn set_signal_detector_to_dac(&mut self) -> Result<(), &'static str> {
        let mut i2c_guard = self.i2c.lock().await;
        self.pca9534
            .write_pin(&mut *i2c_guard, Pin::Pin1, true)
            .await
            .map_err(|_| "Failed to set signal detector to DAC")?;
        Ok(())
    }
}
