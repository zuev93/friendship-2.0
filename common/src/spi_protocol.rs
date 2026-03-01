pub const PACKET_SIZE: usize = 256;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PacketType {
    Idle = 0x00,
    ButtonEvent = 0x01,
    EncoderEvent = 0x02,
    HeadphonesEvent = 0x04,
    LedCommand = 0x10,
    Wm8940Command = 0x12,
    RadioStateCommand = 0x15,
    WaterfallLineCommand = 0x16,
    MenuCommand = 0x17,
    DisplayFpsEvent = 0x05,
}

impl PacketType {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x00 => Some(PacketType::Idle),
            0x01 => Some(PacketType::ButtonEvent),
            0x02 => Some(PacketType::EncoderEvent),
            0x04 => Some(PacketType::HeadphonesEvent),
            0x05 => Some(PacketType::DisplayFpsEvent),
            0x10 => Some(PacketType::LedCommand),
            0x12 => Some(PacketType::Wm8940Command),
            0x15 => Some(PacketType::RadioStateCommand),
            0x16 => Some(PacketType::WaterfallLineCommand),
            0x17 => Some(PacketType::MenuCommand),
            _ => None,
        }
    }
}

pub trait Crc16 {
    fn calculate(&self, data: &[u8]) -> u16;
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

    pub fn set_crc(&mut self, crc: &impl Crc16) {
        let checksum = crc.calculate(&self.data[..PACKET_SIZE - 2]);
        let bytes = checksum.to_be_bytes();
        self.data[PACKET_SIZE - 2] = bytes[0];
        self.data[PACKET_SIZE - 1] = bytes[1];
    }

    pub fn verify_crc(&self, crc: &impl Crc16) -> bool {
        let calculated = crc.calculate(&self.data[..PACKET_SIZE - 2]);
        let stored = u16::from_be_bytes([self.data[PACKET_SIZE - 2], self.data[PACKET_SIZE - 1]]);
        calculated == stored
    }
}

impl Default for Packet {
    fn default() -> Self {
        Self::new()
    }
}

pub trait PacketSerializable: Sized {
    const PACKET_TYPE: PacketType;

    fn write_payload(&self, payload: &mut [u8]);
    fn read_payload(payload: &[u8]) -> Option<Self>;

    fn serialize(&self, packet: &mut Packet, crc: &impl Crc16) {
        packet.set_type(Self::PACKET_TYPE);
        self.write_payload(packet.payload_mut());
        packet.set_crc(crc);
    }

    fn deserialize(packet: &Packet) -> Option<Self> {
        if packet.packet_type() != Some(Self::PACKET_TYPE) {
            return None;
        }
        Self::read_payload(packet.payload())
    }
}
