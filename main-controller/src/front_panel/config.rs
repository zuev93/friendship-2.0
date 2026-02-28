use heapless::index_map::FnvIndexMap;

use crate::front_panel::types::{ButtonFunction, EncoderFunction};

/// Button ID to Function mapping
/// Maps physical button indices (0-11) to their logical functions
pub struct ButtonMapping {
    pub map: FnvIndexMap<u8, ButtonFunction, 16>,
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
    map: FnvIndexMap<u8, EncoderFunction, 16>,
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

        Self { map }
    }

    pub fn get(&self, encoder_id: u8) -> Option<EncoderFunction> {
        self.map.get(&encoder_id).copied()
    }
}

pub fn default_encoder_mapping() -> EncoderMapping {
    EncoderMapping::new()
}
