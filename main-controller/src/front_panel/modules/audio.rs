use common::protocol_types::Wm8940Command;
use embassy_stm32::gpio::Speed;
use embassy_stm32::i2s::{
    ClockPolarity, Config, Format, Mode as I2sMode, Reader, Standard, Writer, I2S,
};
use embassy_stm32::spi::{self, CkPin, MckPin, MisoPin, MosiPin, RxDma, TxDma, WsPin};
use embassy_stm32::time::Hertz;
use embassy_stm32::Peri;
use static_cell::StaticCell;

use crate::app::types::Mode;
use crate::consts::AUDIO_BUFFER_SIZE;
use crate::front_panel::types::ControlBusType;

static TX_BUFFER: StaticCell<[u16; AUDIO_BUFFER_SIZE]> = StaticCell::new();
static RX_BUFFER: StaticCell<[u16; AUDIO_BUFFER_SIZE]> = StaticCell::new();
static AUDIO_I2S: StaticCell<I2S<'static, u16>> = StaticCell::new();

pub struct Audio {
    control_bus: ControlBusType,
    i2s: Option<&'static mut I2S<'static, u16>>,
}

impl Audio {
    pub fn new<T: spi::Instance>(
        control_bus: ControlBusType,
        spi_peri: Peri<'static, T>,
        txsd: Peri<'static, impl MosiPin<T>>,
        rxsd: Peri<'static, impl MisoPin<T>>,
        ws: Peri<'static, impl WsPin<T>>,
        ck: Peri<'static, impl CkPin<T>>,
        mck: Peri<'static, impl MckPin<T>>,
        txdma: Peri<'static, impl TxDma<T>>,
        rxdma: Peri<'static, impl RxDma<T>>,
    ) -> Self {
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
        let i2s = AUDIO_I2S.init(i2s);
        Self {
            control_bus,
            i2s: Some(i2s),
        }
    }

    pub async fn set_volume(&self, volume_percent: u8) -> Result<(), ()> {
        let volume = volume_percent.min(100);

        let wm8940_cmd = Wm8940Command {
            dac_volume_left: volume,
            dac_volume_right: volume,
            adc_volume_left: 0,
            adc_volume_right: 0,
            enable: true,
        };

        let mut spi = self.control_bus.lock().await;
        spi.send(&wm8940_cmd).await.map(|_| ())
    }

    pub async fn set_mode(&mut self, mode: Mode) -> Result<(), &'static str> {
        match mode {
            Mode::WarmUp => self.init(),
            Mode::Rx | Mode::Tx | Mode::StandBy => Ok(()),
        }
    }

    pub fn split_i2s(
        &mut self,
    ) -> (Reader<'static, 'static, u16>, Writer<'static, 'static, u16>) {
        let i2s = self.i2s.take().expect("I2S already split");
        i2s.split().expect("I2S split failed")
    }

    pub fn init(&mut self) -> Result<(), &'static str> {
        self.i2s.as_mut().expect("I2S not available").start();
        Ok(())
    }
}
