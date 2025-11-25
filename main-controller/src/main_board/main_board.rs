/*
 * Main Transceiver Module (async with DMA)
 *
 * High-level transceiver control combining all subsystems.
 */

use embassy_stm32::gpio::Pin;
use embassy_stm32::spi::{self, CkPin, MckPin, MisoPin, MosiPin, RxDma, TxDma, WsPin};
use embassy_stm32::Peri;

use crate::app::types::RfPowerPercent;
use crate::main_board::config::Settings;
use crate::main_board::types::MainBoardI2CMutex;

use super::modules::audio_panel::AudioPanel;
use super::modules::dds::DDS;
use super::modules::filter_select::FilterSelect;
use super::modules::if_gain_control::IfGainControl;
use super::modules::if_reference::IfReference;
use super::modules::power_control::TxPowerControl;
use super::modules::rssi::RssiReader;

pub struct MainBoard {
    pub dds: DDS,
    pub power_control: TxPowerControl,
    pub if_gain_control: IfGainControl,
    pub if_reference: IfReference,
    pub filter_select: FilterSelect,
    pub audio_panel: AudioPanel,
    pub rssi_reader: RssiReader,
}

impl MainBoard {
    pub fn new<T: spi::Instance>(
        i2c: &'static MainBoardI2CMutex,
        fq_ud: Peri<'static, impl Pin>,
        reset: Peri<'static, impl Pin>,
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
            dds: DDS::new(i2c, fq_ud, reset),
            power_control: TxPowerControl::new(i2c, RfPowerPercent::new(settings.tx_power)),
            if_gain_control: IfGainControl::new(i2c),
            if_reference: IfReference::new(i2c, settings.transmit_mode, settings.filter),
            filter_select: FilterSelect::new(i2c, settings.filter),
            // audio_control: AudioControl::new(i2c, settings.af_volume, settings.mic_gain), // Old, removed from modules
            audio_panel: AudioPanel::new(i2c, spi_peri, txsd, rxsd, ws, ck, mck, txdma, rxdma),
            rssi_reader: RssiReader::new(i2c),
        }
    }
}
