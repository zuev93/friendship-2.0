use embassy_stm32::peripherals as stm_peripherals;
use embassy_stm32::sai::{self, Dma, FsPin, Sai, SckPin, SdPin, SubBlockInstance, TxRx};
use embassy_stm32::Peri;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use static_cell::StaticCell;

use crate::app::types::Mode;
use crate::consts::AUDIO_BUFFER_SIZE;

static TX_BUFFER: StaticCell<[u16; AUDIO_BUFFER_SIZE]> = StaticCell::new();
static AUDIO_SAI: StaticCell<
    Mutex<ThreadModeRawMutex, Sai<'static, stm_peripherals::SAI1, u16>>,
> = StaticCell::new();

pub struct Audio {
    sai: &'static Mutex<ThreadModeRawMutex, Sai<'static, stm_peripherals::SAI1, u16>>,
}

impl Audio {
    pub fn new<S: SubBlockInstance>(
        sub_block: sai::SubBlock<'static, stm_peripherals::SAI1, S>,
        sck: Peri<'static, impl SckPin<stm_peripherals::SAI1, S>>,
        sd: Peri<'static, impl SdPin<stm_peripherals::SAI1, S>>,
        fs: Peri<'static, impl FsPin<stm_peripherals::SAI1, S>>,
        dma: Peri<'static, impl Dma<stm_peripherals::SAI1, S>>,
    ) -> Self {
        let tx_buffer = TX_BUFFER.init([0u16; AUDIO_BUFFER_SIZE]);

        let mut config = sai::Config::new();
        config.mode = sai::Mode::Master;
        config.tx_rx = TxRx::Transmitter;
        config.data_size = sai::DataSize::Data16;
        config.stereo_mono = sai::StereoMono::Stereo;
        config.frame_sync_offset = sai::FrameSyncOffset::BeforeFirstBit;
        config.frame_sync_polarity = sai::FrameSyncPolarity::ActiveLow;
        config.frame_sync_active_level_length = sai::word::U7(16);
        config.frame_sync_definition = sai::FrameSyncDefinition::ChannelIdentification;
        config.frame_length = 32;
        config.slot_size = sai::SlotSize::DataSize;
        config.slot_count = sai::word::U4(2);
        config.slot_enable = 0b11;
        config.bit_order = sai::BitOrder::MsbFirst;
        config.clock_strobe = sai::ClockStrobe::Falling;
        config.output_drive = sai::OutputDrive::Immediately;
        config.fifo_threshold = sai::FifoThreshold::ThreeQuarters;

        let sai = Sai::new_asynchronous(sub_block, sck, sd, fs, dma, tx_buffer, config);
        let sai = AUDIO_SAI.init(Mutex::new(sai));
        Self { sai }
    }

    pub async fn set_mode(&self, mode: Mode) -> Result<(), &'static str> {
        match mode {
            Mode::WarmUp => self.init().await,
            Mode::Rx | Mode::Tx | Mode::StandBy => Ok(()),
        }
    }

    pub async fn write(&self, data: &[u16]) -> Result<(), &'static str> {
        let mut sai_guard = self.sai.lock().await;
        sai_guard.write(data).await.map_err(|_| "SAI write failed")
    }

    async fn init(&self) -> Result<(), &'static str> {
        let mut sai_guard = self.sai.lock().await;
        sai_guard.start().map_err(|_| "SAI start failed")
    }
}
