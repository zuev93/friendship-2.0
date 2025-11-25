use heapless::index_map::FnvIndexMap;

use crate::front_panel::types::{ButtonFunction, PotentiometerFunction};

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

        Self { map }
    }

    pub fn get(&self, button_id: u8) -> Option<ButtonFunction> {
        self.map.get(&button_id).copied()
    }
}

pub fn default_button_mapping() -> ButtonMapping {
    ButtonMapping::new()
}

/// Potentiometer ID to Function mapping
/// Maps physical potentiometer indices (0-5) to their logical functions
pub struct PotentiometerMapping {
    map: FnvIndexMap<u8, PotentiometerFunction, 8>,
}

impl PotentiometerMapping {
    pub fn new() -> Self {
        let mut map = FnvIndexMap::new();

        map.insert(0, PotentiometerFunction::Volume).ok();
        map.insert(1, PotentiometerFunction::Microphone).ok();
        map.insert(2, PotentiometerFunction::RfPower).ok();
        map.insert(3, PotentiometerFunction::IfGain).ok();
        map.insert(4, PotentiometerFunction::Clarifier).ok();
        map.insert(5, PotentiometerFunction::Squelch).ok();

        Self { map }
    }

    pub fn get(&self, pot_id: u8) -> Option<PotentiometerFunction> {
        self.map.get(&pot_id).copied()
    }
}

pub fn default_potentiometer_mapping() -> PotentiometerMapping {
    PotentiometerMapping::new()
}
