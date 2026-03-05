use crate::app::types::Mode;
use crate::consts::ADC_BUFFER_SIZE;
use crate::i2c_map::I2cAddress;
use crate::main_board::types::{MainBoardI2C, MainBoardI2CMutex};
use common::drivers::cs4272::Cs4272;
use common::drivers::pca9534::{Pin, PCA9534};

use embassy_stm32::peripherals as stm_peripherals;
use embassy_stm32::sai::{self, Dma, FsPin, Sai, SckPin, SdPin, TxRx};
use embassy_stm32::Peri;
use static_cell::StaticCell;

static TX_BUFFER: StaticCell<[u32; ADC_BUFFER_SIZE]> = StaticCell::new();
static RX_BUFFER: StaticCell<[u32; ADC_BUFFER_SIZE]> = StaticCell::new();
static SAI_TX: StaticCell<Sai<'static, stm_peripherals::SAI1, u32>> = StaticCell::new();
static SAI_RX: StaticCell<Sai<'static, stm_peripherals::SAI1, u32>> = StaticCell::new();

pub struct AudioPanel {
    audio_codec: Cs4272<MainBoardI2C>,
    io: PCA9534<MainBoardI2C>,
    sai_tx: Option<&'static mut Sai<'static, stm_peripherals::SAI1, u32>>,
    sai_rx: Option<&'static mut Sai<'static, stm_peripherals::SAI1, u32>>,
}

impl AudioPanel {
    pub fn new(
        i2c: &'static MainBoardI2CMutex,
        cs4272_addr: I2cAddress,
        pca9534_addr: I2cAddress,
        sub_block_a: sai::SubBlock<'static, stm_peripherals::SAI1, sai::A>,
        sub_block_b: sai::SubBlock<'static, stm_peripherals::SAI1, sai::B>,
        sck: Peri<'static, impl SckPin<stm_peripherals::SAI1, sai::A>>,
        sd_a: Peri<'static, impl SdPin<stm_peripherals::SAI1, sai::A>>,
        sd_b: Peri<'static, impl SdPin<stm_peripherals::SAI1, sai::B>>,
        fs: Peri<'static, impl FsPin<stm_peripherals::SAI1, sai::A>>,
        dma_a: Peri<'static, impl Dma<stm_peripherals::SAI1, sai::A>>,
        dma_b: Peri<'static, impl Dma<stm_peripherals::SAI1, sai::B>>,
    ) -> Self {
        let cs4272 = Cs4272::new(cs4272_addr.into(), i2c);
        let pca9534 = PCA9534::new(pca9534_addr.into(), i2c);

        let mut tx_config = sai::Config::new();
        tx_config.mode = sai::Mode::Slave;
        tx_config.tx_rx = TxRx::Transmitter;
        tx_config.sync_output = true;
        tx_config.data_size = sai::DataSize::Data24;
        tx_config.stereo_mono = sai::StereoMono::Stereo;
        tx_config.frame_sync_offset = sai::FrameSyncOffset::BeforeFirstBit;
        tx_config.frame_sync_polarity = sai::FrameSyncPolarity::ActiveLow;
        tx_config.frame_sync_active_level_length = sai::word::U7(32);
        tx_config.frame_sync_definition = sai::FrameSyncDefinition::ChannelIdentification;
        tx_config.frame_length = 64;
        tx_config.slot_size = sai::SlotSize::Channel32;
        tx_config.slot_count = sai::word::U4(2);
        tx_config.slot_enable = 0b11;
        tx_config.bit_order = sai::BitOrder::MsbFirst;
        tx_config.clock_strobe = sai::ClockStrobe::Falling;
        tx_config.output_drive = sai::OutputDrive::Immediately;
        tx_config.fifo_threshold = sai::FifoThreshold::ThreeQuarters;

        let mut rx_config = sai::Config::new();
        rx_config.tx_rx = TxRx::Receiver;
        rx_config.data_size = sai::DataSize::Data24;
        rx_config.stereo_mono = sai::StereoMono::Stereo;
        rx_config.frame_sync_offset = sai::FrameSyncOffset::BeforeFirstBit;
        rx_config.frame_sync_polarity = sai::FrameSyncPolarity::ActiveLow;
        rx_config.frame_sync_active_level_length = sai::word::U7(32);
        rx_config.frame_sync_definition = sai::FrameSyncDefinition::ChannelIdentification;
        rx_config.frame_length = 64;
        rx_config.slot_size = sai::SlotSize::Channel32;
        rx_config.slot_count = sai::word::U4(2);
        rx_config.slot_enable = 0b11;
        rx_config.bit_order = sai::BitOrder::MsbFirst;
        rx_config.clock_strobe = sai::ClockStrobe::Falling;
        rx_config.fifo_threshold = sai::FifoThreshold::ThreeQuarters;

        let tx_buffer = TX_BUFFER.init([0u32; ADC_BUFFER_SIZE]);
        let rx_buffer = RX_BUFFER.init([0u32; ADC_BUFFER_SIZE]);

        let sai_tx = Sai::new_asynchronous(
            sub_block_a, sck, sd_a, fs, dma_a, tx_buffer, tx_config,
        );
        let sai_rx = Sai::new_synchronous(sub_block_b, sd_b, dma_b, rx_buffer, rx_config);

        let sai_tx = SAI_TX.init(sai_tx);
        let sai_rx = SAI_RX.init(sai_rx);

        Self {
            audio_codec: cs4272,
            io: pca9534,
            sai_tx: Some(sai_tx),
            sai_rx: Some(sai_rx),
        }
    }

    pub async fn set_mode(&mut self, mode: Mode) -> Result<(), &'static str> {
        match mode {
            Mode::Rx => {
                self.set_mute(false).await?;
                self.set_signal_detector_to_adc().await
            }
            Mode::Tx => {
                self.set_signal_detector_to_dac().await
            }
            Mode::WarmUp => {
                self.init().await?;
                self.set_hpf(false).await
            }
            Mode::StandBy => {
                self.set_mute(true).await
            }
        }
    }

    pub fn split_sai(
        &mut self,
    ) -> (
        &'static mut Sai<'static, stm_peripherals::SAI1, u32>,
        &'static mut Sai<'static, stm_peripherals::SAI1, u32>,
    ) {
        let tx = self.sai_tx.take().expect("SAI TX already split");
        let rx = self.sai_rx.take().expect("SAI RX already split");
        (tx, rx)
    }

    pub async fn init(&mut self) -> Result<(), &'static str> {
        self.io
            .init()
            .await
            .map_err(|_| "Failed to initialize PCA9534")?;

        self.io
            .set_pin_direction(Pin::Pin0, false)
            .await
            .map_err(|_| "Failed to configure P0 as output")?;

        self.io
            .set_pin_direction(Pin::Pin1, false)
            .await
            .map_err(|_| "Failed to configure P1 as output")?;

        self.reset_codec().await?;

        self.audio_codec
            .init()
            .await
            .map_err(|_| "Failed to initialize CS4272")?;

        self.sai_rx
            .as_mut()
            .expect("SAI RX not available")
            .start()
            .map_err(|_| "SAI RX start failed")?;

        Ok(())
    }

    async fn reset_codec(&mut self) -> Result<(), &'static str> {
        self.io
            .write_pin(Pin::Pin0, false)
            .await
            .map_err(|_| "Failed to set CS4272 reset low")?;

        embassy_time::Timer::after_millis(10).await;

        self.io
            .write_pin(Pin::Pin0, true)
            .await
            .map_err(|_| "Failed to set CS4272 reset high")?;

        Ok(())
    }

    pub async fn set_dac_volume(&mut self, attenuation_half_db: u8) -> Result<(), &'static str> {
        self.audio_codec
            .set_dac_volume(attenuation_half_db)
            .await
            .map_err(|_| "Failed to set DAC volume")
    }

    pub async fn set_mute(&mut self, mute: bool) -> Result<(), &'static str> {
        self.audio_codec
            .set_mute(mute)
            .await
            .map_err(|_| "Failed to set DAC mute")
    }

    pub async fn set_hpf(&mut self, enabled: bool) -> Result<(), &'static str> {
        self.audio_codec
            .set_hpf(enabled)
            .await
            .map_err(|_| "Failed to set HPF")
    }

    async fn set_signal_detector_to_adc(&mut self) -> Result<(), &'static str> {
        self.io
            .write_pin(Pin::Pin1, false)
            .await
            .map_err(|_| "Failed to set signal detector to ADC")?;
        Ok(())
    }

    async fn set_signal_detector_to_dac(&mut self) -> Result<(), &'static str> {
        self.io
            .write_pin(Pin::Pin1, true)
            .await
            .map_err(|_| "Failed to set signal detector to DAC")?;
        Ok(())
    }
}
