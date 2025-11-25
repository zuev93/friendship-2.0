use crate::consts::AUDIO_BUFFER_SIZE;
use crate::i2c_map;
use crate::main_board::types::MainBoardI2CMutex;
use common::drivers::pca9534::{Pin, PCA9534};
use common::drivers::pcm3060::Pcm3060;
use embassy_stm32::gpio::Speed;
use embassy_stm32::i2s::{ClockPolarity, Config, Format, Mode, Reader, Standard, Writer, I2S};
use embassy_stm32::spi::{self, CkPin, MckPin, MisoPin, MosiPin, RxDma, TxDma, WsPin};
use embassy_stm32::time::Hertz;
use embassy_stm32::Peri;
use static_cell::StaticCell;

const PCA9534_AUDIO_PANEL_ADDR: u8 = 0x20;
static TX_BUFFER: StaticCell<[u16; AUDIO_BUFFER_SIZE]> = StaticCell::new();
static RX_BUFFER: StaticCell<[u16; AUDIO_BUFFER_SIZE]> = StaticCell::new();

pub struct AudioPanel {
    pcm3060: Pcm3060,
    pca9534: PCA9534,
    i2c: &'static MainBoardI2CMutex,
    // TODO check pcm3060
    i2s: I2S<'static, u16>,
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
        config.mode = Mode::Master;
        config.standard = Standard::Philips;
        config.format = Format::Data16Channel16;
        config.clock_polarity = ClockPolarity::IdleLow;
        config.master_clock = false;

        let tx_buffer = TX_BUFFER.init([0u16; AUDIO_BUFFER_SIZE]);
        let rx_buffer = RX_BUFFER.init([0u16; AUDIO_BUFFER_SIZE]);

        let i2s = I2S::new_full_duplex(
            spi_peri, txsd, rxsd, ws, ck, mck, txdma, tx_buffer, rxdma, rx_buffer, config,
        );

        Self {
            pcm3060,
            pca9534,
            i2c,
            i2s: i2s,
        }
    }

    pub fn split_i2s(&mut self) -> (Reader<'_, 'static, u16>, Writer<'_, 'static, u16>) {
        self.i2s.read()
        self.i2s.split().unwrap()
    }

    #[allow(dead_code)]
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

        self.i2s.start();

        Ok(())
    }

    pub async fn reset_pcm3060(&mut self) -> Result<(), &'static str> {
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

    pub async fn set_signal_detector_to_adc(&mut self) -> Result<(), &'static str> {
        let mut i2c_guard = self.i2c.lock().await;
        self.pca9534
            .write_pin(&mut *i2c_guard, Pin::Pin1, false)
            .await
            .map_err(|_| "Failed to set signal detector to ADC")?;
        Ok(())
    }

    pub async fn set_signal_detector_to_dac(&mut self) -> Result<(), &'static str> {
        let mut i2c_guard = self.i2c.lock().await;
        self.pca9534
            .write_pin(&mut *i2c_guard, Pin::Pin1, true)
            .await
            .map_err(|_| "Failed to set signal detector to DAC")?;
        Ok(())
    }
}
