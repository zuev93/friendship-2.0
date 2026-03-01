use heapless::index_map::FnvIndexMap;

use crate::front_panel::types::{ButtonFunction, EncoderFunction};

/// Button ID to Function mapping
/// Maps physical button indices (0-11) to their logical functions
pub struct ButtonMapping {
    pub map: FnvIndexMap<u8, ButtonFunction, 32>,
}

impl ButtonMapping {
    pub fn new() -> Self {
        let mut map = FnvIndexMap::new();

        // Main buttons (0-6)
        map.insert(0, ButtonFunction::Power).ok();
        map.insert(1, ButtonFunction::Transmit).ok();
        map.insert(2, ButtonFunction::Tone).ok();
        map.insert(3, ButtonFunction::TransmitMode).ok();
        map.insert(4, ButtonFunction::Rit).ok();
        map.insert(5, ButtonFunction::RfGain).ok();
        map.insert(6, ButtonFunction::Agc).ok();

        // ICOM interface buttons (7-9)
        map.insert(7, ButtonFunction::IcomPtt).ok();
        map.insert(8, ButtonFunction::IcomSql).ok();
        map.insert(9, ButtonFunction::IcomUpDown).ok();

        // Encoder buttons (10-11)
        map.insert(10, ButtonFunction::Cancel).ok();
        map.insert(11, ButtonFunction::Ok).ok();

        map.insert(12, ButtonFunction::NoiseBlanker).ok();
        map.insert(13, ButtonFunction::Filter).ok();
        map.insert(14, ButtonFunction::NoiseReduction).ok();
        map.insert(15, ButtonFunction::AutoNotch).ok();
        map.insert(16, ButtonFunction::CwPeak).ok();
        map.insert(17, ButtonFunction::AudioAgc).ok();
        map.insert(18, ButtonFunction::DspFilter).ok();
        map.insert(19, ButtonFunction::TxEqualizer).ok();
        map.insert(20, ButtonFunction::RxEqualizer).ok();

        Self { map }
    }

    pub fn get(&self, button_id: u8) -> Option<ButtonFunction> {
        self.map.get(&button_id).copied()
    }
}

pub fn default_button_mapping() -> ButtonMapping {
    ButtonMapping::new()
}

pub struct EncoderMapping {
    map: FnvIndexMap<u8, EncoderFunction, 32>,
}

impl EncoderMapping {
    pub fn new() -> Self {
        let mut map = FnvIndexMap::new();

        map.insert(0, EncoderFunction::Band).ok();
        map.insert(1, EncoderFunction::Vfo).ok();
        map.insert(2, EncoderFunction::Volume).ok();
        map.insert(3, EncoderFunction::RfPower).ok();
        map.insert(4, EncoderFunction::Microphone).ok();
        map.insert(5, EncoderFunction::IfGain).ok();
        map.insert(6, EncoderFunction::Clarifier).ok();
        map.insert(7, EncoderFunction::Squelch).ok();
        map.insert(8, EncoderFunction::Menu).ok();
        map.insert(9, EncoderFunction::NbLevel).ok();
        map.insert(10, EncoderFunction::Compression).ok();
        map.insert(11, EncoderFunction::NrLevel).ok();
        map.insert(12, EncoderFunction::DspBandwidth).ok();
        map.insert(13, EncoderFunction::DspShift).ok();
        map.insert(14, EncoderFunction::CwPeakWidth).ok();
        map.insert(15, EncoderFunction::CwPitch).ok();
        map.insert(16, EncoderFunction::TxEqLow).ok();
        map.insert(17, EncoderFunction::TxEqMid).ok();
        map.insert(18, EncoderFunction::TxEqHigh).ok();
        map.insert(19, EncoderFunction::RxEqLow).ok();
        map.insert(20, EncoderFunction::RxEqMid).ok();
        map.insert(21, EncoderFunction::RxEqHigh).ok();

        Self { map }
    }

    pub fn get(&self, encoder_id: u8) -> Option<EncoderFunction> {
        self.map.get(&encoder_id).copied()
    }
}

pub fn default_encoder_mapping() -> EncoderMapping {
    EncoderMapping::new()
}
