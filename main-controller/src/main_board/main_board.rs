/*
 * Main Transceiver Module (async with DMA)
 *
 * High-level transceiver control combining all subsystems.
 */

use embassy_stm32::spi::{self, CkPin, MckPin, MisoPin, MosiPin, RxDma, TxDma, WsPin};
use embassy_stm32::Peri;

use crate::main_board::config::Settings;
use crate::main_board::modules::crystall_filter::CrystallFilter;
use crate::main_board::types::MainBoardI2CMutex;

use super::modules::audio_panel::AudioPanel;
use super::modules::dds::DDS;
use super::modules::if_gain_control::IfGainControl;
use super::modules::if_reference::IfReference;

pub struct MainBoard {
    pub dds: DDS,
    pub crystall_filter: CrystallFilter,
    pub if_gain_control: IfGainControl,
    pub if_reference: IfReference,
    pub audio_panel: AudioPanel,
}

impl MainBoard {
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
        let settings = Settings::load();

        Self {
            dds: DDS::new(i2c),
            crystall_filter: CrystallFilter::new(i2c),
            if_gain_control: IfGainControl::new(i2c),
            if_reference: IfReference::new(i2c, settings.transmit_mode, settings.filter),
            // audio_control: AudioControl::new(i2c, settings.af_volume, settings.mic_gain), // Old, removed from modules
            audio_panel: AudioPanel::new(i2c, spi_peri, txsd, rxsd, ws, ck, mck, txdma, rxdma),
        }
    }
}
