use crate::spi_protocol::{Packet, PacketType};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ButtonState {
    Released,
    Pressed,
}

#[derive(Debug, Clone, Copy)]
pub struct ButtonEvent {
    pub id: u8,
    pub state: ButtonState,
}

impl ButtonEvent {
    pub fn serialize(&self, packet: &mut Packet) {
        packet.set_type(PacketType::ButtonEvent);
        let payload = packet.payload_mut();
        payload[0] = self.id;
        payload[1] = match self.state {
            ButtonState::Pressed => 1,
            ButtonState::Released => 0,
        };
        packet.set_crc();
    }

    pub fn deserialize(packet: &Packet) -> Option<Self> {
        if packet.packet_type() != Some(PacketType::ButtonEvent) {
            return None;
        }
        let payload = packet.payload();
        Some(Self {
            id: payload[0],
            state: if payload[1] == 1 {
                ButtonState::Pressed
            } else {
                ButtonState::Released
            },
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EncoderDirection {
    Clockwise,
    CounterClockwise,
}

#[derive(Debug, Clone, Copy)]
pub struct EncoderEvent {
    pub id: u8,
    pub direction: EncoderDirection,
    pub steps: i8,
}

impl EncoderEvent {
    pub fn serialize(&self, packet: &mut Packet) {
        packet.set_type(PacketType::EncoderEvent);
        let payload = packet.payload_mut();
        payload[0] = self.id;
        payload[1] = match self.direction {
            EncoderDirection::Clockwise => 1,
            EncoderDirection::CounterClockwise => 0,
        };
        payload[2] = self.steps as u8;
        packet.set_crc();
    }

    pub fn deserialize(packet: &Packet) -> Option<Self> {
        if packet.packet_type() != Some(PacketType::EncoderEvent) {
            return None;
        }
        let payload = packet.payload();
        Some(Self {
            id: payload[0],
            direction: if payload[1] == 1 {
                EncoderDirection::Clockwise
            } else {
                EncoderDirection::CounterClockwise
            },
            steps: payload[2] as i8,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PotentiometerValue {
    pub id: u8,
    pub value: u16,
}

impl PotentiometerValue {
    pub fn serialize(&self, packet: &mut Packet) {
        packet.set_type(PacketType::PotentiometerValue);
        let payload = packet.payload_mut();
        payload[0] = self.id;
        payload[1] = (self.value >> 8) as u8;
        payload[2] = self.value as u8;
        packet.set_crc();
    }

    pub fn deserialize(packet: &Packet) -> Option<Self> {
        if packet.packet_type() != Some(PacketType::PotentiometerValue) {
            return None;
        }
        let payload = packet.payload();
        Some(Self {
            id: payload[0],
            value: ((payload[1] as u16) << 8) | (payload[2] as u16),
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HeadphonesEvent {
    pub connected: bool,
}

impl HeadphonesEvent {
    pub fn serialize(&self, packet: &mut Packet) {
        packet.set_type(PacketType::HeadphonesEvent);
        let payload = packet.payload_mut();
        payload[0] = if self.connected { 1 } else { 0 };
        packet.set_crc();
    }

    pub fn deserialize(packet: &Packet) -> Option<Self> {
        if packet.packet_type() != Some(PacketType::HeadphonesEvent) {
            return None;
        }
        let payload = packet.payload();
        Some(Self {
            connected: payload[0] != 0,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LedState {
    Off = 0,
    Red = 1,
    Green = 2,
}

#[derive(Debug, Clone, Copy)]
pub struct LedCommand {
    pub led_id: u8,
    pub state: LedState,
}

impl LedCommand {
    pub fn serialize(&self, packet: &mut Packet) {
        packet.set_type(PacketType::LedCommand);
        let payload = packet.payload_mut();
        payload[0] = self.led_id;
        payload[1] = self.state as u8;
        packet.set_crc();
    }

    pub fn deserialize(packet: &Packet) -> Option<Self> {
        if packet.packet_type() != Some(PacketType::LedCommand) {
            return None;
        }
        let payload = packet.payload();
        let state = match payload[1] {
            0 => LedState::Off,
            1 => LedState::Red,
            2 => LedState::Green,
            _ => LedState::Off,
        };
        Some(Self {
            led_id: payload[0],
            state,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SMeterCommand {
    pub value: u16,
}

impl SMeterCommand {
    pub fn serialize(&self, packet: &mut Packet) {
        packet.set_type(PacketType::SMeterCommand);
        let payload = packet.payload_mut();
        payload[0] = (self.value >> 8) as u8;
        payload[1] = self.value as u8;
        packet.set_crc();
    }

    pub fn deserialize(packet: &Packet) -> Option<Self> {
        if packet.packet_type() != Some(PacketType::SMeterCommand) {
            return None;
        }
        let payload = packet.payload();
        Some(Self {
            value: ((payload[0] as u16) << 8) | (payload[1] as u16),
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Wm8940Command {
    pub dac_volume_left: u8,
    pub dac_volume_right: u8,
    pub adc_volume_left: u8,
    pub adc_volume_right: u8,
    pub enable: bool,
}

impl Wm8940Command {
    pub fn serialize(&self, packet: &mut Packet) {
        packet.set_type(PacketType::Wm8940Command);
        let payload = packet.payload_mut();
        payload[0] = self.dac_volume_left;
        payload[1] = self.dac_volume_right;
        payload[2] = self.adc_volume_left;
        payload[3] = self.adc_volume_right;
        payload[4] = if self.enable { 1 } else { 0 };
        packet.set_crc();
    }

    pub fn deserialize(packet: &Packet) -> Option<Self> {
        if packet.packet_type() != Some(PacketType::Wm8940Command) {
            return None;
        }
        let payload = packet.payload();
        Some(Self {
            dac_volume_left: payload[0],
            dac_volume_right: payload[1],
            adc_volume_left: payload[2],
            adc_volume_right: payload[3],
            enable: payload[4] != 0,
        })
    }
}

pub const DISPLAY_BUFFER_SIZE: usize = 128 * 64 / 8;

#[derive(Debug, Clone)]
pub struct DisplayCommand {
    pub display_id: u8,
    pub buffer: [u8; DISPLAY_BUFFER_SIZE],
    pub dirty: bool,
}

impl DisplayCommand {
    pub fn serialize(&self, packet: &mut Packet) {
        packet.set_type(PacketType::DisplayCommand);
        let payload = packet.payload_mut();
        payload[0] = self.display_id;
        let data_len = DISPLAY_BUFFER_SIZE.min(payload.len() - 3);
        payload[1] = (data_len >> 8) as u8;
        payload[2] = data_len as u8;
        payload[3..3 + data_len].copy_from_slice(&self.buffer[..data_len]);
        packet.set_crc();
    }

    pub fn deserialize(packet: &Packet) -> Option<Self> {
        if packet.packet_type() != Some(PacketType::DisplayCommand) {
            return None;
        }
        let payload = packet.payload();
        let display_id = payload[0];
        let data_len = ((payload[1] as usize) << 8) | (payload[2] as usize);
        let mut buffer = [0u8; DISPLAY_BUFFER_SIZE];
        let copy_len = data_len.min(buffer.len()).min(payload.len() - 3);
        buffer[..copy_len].copy_from_slice(&payload[3..3 + copy_len]);
        Some(Self {
            display_id,
            buffer,
            dirty: true,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DisplayEnableCommand {
    pub enabled: bool,
}

impl DisplayEnableCommand {
    pub fn serialize(&self, packet: &mut Packet) {
        packet.set_type(PacketType::DisplayEnableCommand);
        let payload = packet.payload_mut();
        payload[0] = if self.enabled { 1 } else { 0 };
        packet.set_crc();
    }

    pub fn deserialize(packet: &Packet) -> Option<Self> {
        if packet.packet_type() != Some(PacketType::DisplayEnableCommand) {
            return None;
        }
        let payload = packet.payload();
        Some(Self {
            enabled: payload[0] != 0,
        })
    }
}
