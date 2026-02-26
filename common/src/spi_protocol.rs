pub const PACKET_SIZE: usize = 256;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PacketType {
    Idle = 0x00,
    ButtonEvent = 0x01,
    EncoderEvent = 0x02,
    HeadphonesEvent = 0x04,
    LedCommand = 0x10,
    SMeterCommand = 0x11,
    Wm8940Command = 0x12,
    DisplayCommand = 0x13,
    DisplayEnableCommand = 0x14,
}

impl PacketType {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x00 => Some(PacketType::Idle),
            0x01 => Some(PacketType::ButtonEvent),
            0x02 => Some(PacketType::EncoderEvent),
            0x04 => Some(PacketType::HeadphonesEvent),
            0x10 => Some(PacketType::LedCommand),
            0x11 => Some(PacketType::SMeterCommand),
            0x12 => Some(PacketType::Wm8940Command),
            0x13 => Some(PacketType::DisplayCommand),
            0x14 => Some(PacketType::DisplayEnableCommand),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Packet {
    pub data: [u8; PACKET_SIZE],
}

impl Packet {
    pub const fn new() -> Self {
        Self {
            data: [0; PACKET_SIZE],
        }
    }

    pub fn packet_type(&self) -> Option<PacketType> {
        PacketType::from_u8(self.data[0])
    }

    pub fn set_type(&mut self, packet_type: PacketType) {
        self.data[0] = packet_type as u8;
    }

    pub fn payload(&self) -> &[u8] {
        &self.data[1..PACKET_SIZE - 2]
    }

    pub fn payload_mut(&mut self) -> &mut [u8] {
        &mut self.data[1..PACKET_SIZE - 2]
    }

    pub fn set_crc(&mut self) {
        let crc = Self::calculate_crc(&self.data[..PACKET_SIZE - 2]);
        self.data[PACKET_SIZE - 2] = (crc >> 8) as u8;
        self.data[PACKET_SIZE - 1] = crc as u8;
    }

    pub fn verify_crc(&self) -> bool {
        let calculated = Self::calculate_crc(&self.data[..PACKET_SIZE - 2]);
        let stored =
            ((self.data[PACKET_SIZE - 2] as u16) << 8) | (self.data[PACKET_SIZE - 1] as u16);
        calculated == stored
    }

    fn calculate_crc(data: &[u8]) -> u16 {
        let mut crc: u16 = 0xFFFF;
        for &byte in data {
            crc ^= byte as u16;
            for _ in 0..8 {
                if crc & 0x0001 != 0 {
                    crc = (crc >> 1) ^ 0xA001;
                } else {
                    crc >>= 1;
                }
            }
        }
        crc
    }
}

impl Default for Packet {
    fn default() -> Self {
        Self::new()
    }
}
