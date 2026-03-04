use embassy_stm32::spi::{self, CkPin, MckPin, MisoPin, MosiPin, RxDma, TxDma, WsPin};
use embassy_stm32::Peri;

use crate::i2c_map::MainI2cMap;
use crate::main_board::modules::crystal_filter::CrystalFilter;
use crate::main_board::modules::detector::Detector;
use crate::main_board::types::MainBoardI2CMutex;

use super::modules::audio_panel::AudioPanel;
use super::modules::if_amplifier::IfAmplifier;
use super::modules::mixer::Mixer;

pub struct MainBoard {
    pub mixer: Mixer,
    pub crystal_filter: CrystalFilter,
    pub if_amplifier: IfAmplifier,
    pub detector: Detector,
    pub audio_panel: AudioPanel,
}

impl MainBoard {
    pub fn new<T: spi::Instance>(
        i2c: &'static MainBoardI2CMutex,
        i2c_map: MainI2cMap,
        spi_peri: Peri<'static, T>,
        txsd: Peri<'static, impl MosiPin<T>>,
        rxsd: Peri<'static, impl MisoPin<T>>,
        ws: Peri<'static, impl WsPin<T>>,
        ck: Peri<'static, impl CkPin<T>>,
        mck: Peri<'static, impl MckPin<T>>,
        txdma: Peri<'static, impl TxDma<T>>,
        rxdma: Peri<'static, impl RxDma<T>>,
    ) -> Self {
        let MainI2cMap {
            mixer_sc18is602,
            filter_pca9534,
            filter_mcp4725,
            if_amp_pca9534,
            if_amp_ads1015_rssi,
            if_amp_mcp4725,
            detector_sc18is602,
            audio_pcm3060,
            audio_panel_pca9534,
        } = i2c_map;

        Self {
            mixer: Mixer::new(i2c, mixer_sc18is602),
            crystal_filter: CrystalFilter::new(i2c, filter_mcp4725, filter_pca9534),
            if_amplifier: IfAmplifier::new(
                i2c,
                if_amp_mcp4725,
                if_amp_ads1015_rssi,
                if_amp_pca9534,
            ),
            detector: Detector::new(i2c, detector_sc18is602),
            audio_panel: AudioPanel::new(
                i2c,
                audio_pcm3060,
                audio_panel_pca9534,
                spi_peri,
                txsd,
                rxsd,
                ws,
                ck,
                mck,
                txdma,
                rxdma,
            ),
        }
    }
}
