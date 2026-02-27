use common::protocol_types::Wm8940Command;
use embassy_stm32::peripherals as stm_peripherals;
use embassy_stm32::sai::{self, Dma, FsPin, Sai, SckPin, SdPin, TxRx};
use embassy_stm32::Peri;
use static_cell::StaticCell;

use crate::app::types::Mode;
use crate::consts::AUDIO_BUFFER_SIZE;
use crate::front_panel::types::ControlBusType;

static TX_BUFFER: StaticCell<[u16; AUDIO_BUFFER_SIZE]> = StaticCell::new();
static RX_BUFFER: StaticCell<[u16; AUDIO_BUFFER_SIZE]> = StaticCell::new();
static SAI_TX: StaticCell<Sai<'static, stm_peripherals::SAI2, u16>> = StaticCell::new();
static SAI_RX: StaticCell<Sai<'static, stm_peripherals::SAI2, u16>> = StaticCell::new();

pub struct Audio {
    control_bus: ControlBusType,
    sai_tx: Option<&'static mut Sai<'static, stm_peripherals::SAI2, u16>>,
    sai_rx: Option<&'static mut Sai<'static, stm_peripherals::SAI2, u16>>,
}

impl Audio {
    pub fn new(
        control_bus: ControlBusType,
        sub_block_b: sai::SubBlock<'static, stm_peripherals::SAI2, sai::B>,
        sub_block_a: sai::SubBlock<'static, stm_peripherals::SAI2, sai::A>,
        sck: Peri<'static, impl SckPin<stm_peripherals::SAI2, sai::B>>,
        sd_b: Peri<'static, impl SdPin<stm_peripherals::SAI2, sai::B>>,
        sd_a: Peri<'static, impl SdPin<stm_peripherals::SAI2, sai::A>>,
        fs: Peri<'static, impl FsPin<stm_peripherals::SAI2, sai::B>>,
        dma_b: Peri<'static, impl Dma<stm_peripherals::SAI2, sai::B>>,
        dma_a: Peri<'static, impl Dma<stm_peripherals::SAI2, sai::A>>,
    ) -> Self {
        let mut tx_config = sai::Config::new();
        tx_config.mode = sai::Mode::Master;
        tx_config.tx_rx = TxRx::Transmitter;
        tx_config.sync_output = true;
        tx_config.data_size = sai::DataSize::Data16;
        tx_config.stereo_mono = sai::StereoMono::Stereo;
        tx_config.frame_sync_offset = sai::FrameSyncOffset::BeforeFirstBit;
        tx_config.frame_sync_polarity = sai::FrameSyncPolarity::ActiveLow;
        tx_config.frame_sync_active_level_length = sai::word::U7(16);
        tx_config.frame_sync_definition = sai::FrameSyncDefinition::ChannelIdentification;
        tx_config.frame_length = 32;
        tx_config.slot_size = sai::SlotSize::DataSize;
        tx_config.slot_count = sai::word::U4(2);
        tx_config.slot_enable = 0b11;
        tx_config.bit_order = sai::BitOrder::MsbFirst;
        tx_config.clock_strobe = sai::ClockStrobe::Falling;
        tx_config.output_drive = sai::OutputDrive::Immediately;
        tx_config.fifo_threshold = sai::FifoThreshold::ThreeQuarters;

        let mut rx_config = sai::Config::new();
        rx_config.tx_rx = TxRx::Receiver;
        rx_config.data_size = sai::DataSize::Data16;
        rx_config.stereo_mono = sai::StereoMono::Stereo;
        rx_config.frame_sync_offset = sai::FrameSyncOffset::BeforeFirstBit;
        rx_config.frame_sync_polarity = sai::FrameSyncPolarity::ActiveLow;
        rx_config.frame_sync_active_level_length = sai::word::U7(16);
        rx_config.frame_sync_definition = sai::FrameSyncDefinition::ChannelIdentification;
        rx_config.frame_length = 32;
        rx_config.slot_size = sai::SlotSize::DataSize;
        rx_config.slot_count = sai::word::U4(2);
        rx_config.slot_enable = 0b11;
        rx_config.bit_order = sai::BitOrder::MsbFirst;
        rx_config.clock_strobe = sai::ClockStrobe::Falling;
        rx_config.fifo_threshold = sai::FifoThreshold::ThreeQuarters;

        let tx_buffer = TX_BUFFER.init([0u16; AUDIO_BUFFER_SIZE]);
        let rx_buffer = RX_BUFFER.init([0u16; AUDIO_BUFFER_SIZE]);

        let sai_tx = Sai::new_asynchronous(
            sub_block_b, sck, sd_b, fs, dma_b, tx_buffer, tx_config,
        );
        let sai_rx = Sai::new_synchronous(sub_block_a, sd_a, dma_a, rx_buffer, rx_config);

        let sai_tx = SAI_TX.init(sai_tx);
        let sai_rx = SAI_RX.init(sai_rx);

        Self {
            control_bus,
            sai_tx: Some(sai_tx),
            sai_rx: Some(sai_rx),
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

    pub fn split_sai(
        &mut self,
    ) -> (
        &'static mut Sai<'static, stm_peripherals::SAI2, u16>,
        &'static mut Sai<'static, stm_peripherals::SAI2, u16>,
    ) {
        let tx = self.sai_tx.take().expect("SAI TX already split");
        let rx = self.sai_rx.take().expect("SAI RX already split");
        (tx, rx)
    }

    pub fn init(&mut self) -> Result<(), &'static str> {
        self.sai_rx
            .as_mut()
            .expect("SAI RX not available")
            .start()
            .map_err(|_| "SAI RX start failed")?;
        Ok(())
    }
}
