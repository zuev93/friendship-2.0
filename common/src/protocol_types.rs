use crate::spi_protocol::{PacketSerializable, PacketType};

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

impl PacketSerializable for ButtonEvent {
    const PACKET_TYPE: PacketType = PacketType::ButtonEvent;

    fn write_payload(&self, payload: &mut [u8]) {
        payload[0] = self.id;
        payload[1] = match self.state {
            ButtonState::Pressed => 1,
            ButtonState::Released => 0,
        };
    }

    fn read_payload(payload: &[u8]) -> Option<Self> {
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

impl PacketSerializable for EncoderEvent {
    const PACKET_TYPE: PacketType = PacketType::EncoderEvent;

    fn write_payload(&self, payload: &mut [u8]) {
        payload[0] = self.id;
        payload[1] = match self.direction {
            EncoderDirection::Clockwise => 1,
            EncoderDirection::CounterClockwise => 0,
        };
        payload[2] = self.steps as u8;
    }

    fn read_payload(payload: &[u8]) -> Option<Self> {
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
pub struct HeadphonesEvent {
    pub connected: bool,
}

impl PacketSerializable for HeadphonesEvent {
    const PACKET_TYPE: PacketType = PacketType::HeadphonesEvent;

    fn write_payload(&self, payload: &mut [u8]) {
        payload[0] = if self.connected { 1 } else { 0 };
    }

    fn read_payload(payload: &[u8]) -> Option<Self> {
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

impl PacketSerializable for LedCommand {
    const PACKET_TYPE: PacketType = PacketType::LedCommand;

    fn write_payload(&self, payload: &mut [u8]) {
        payload[0] = self.led_id;
        payload[1] = self.state as u8;
    }

    fn read_payload(payload: &[u8]) -> Option<Self> {
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

impl PacketSerializable for SMeterCommand {
    const PACKET_TYPE: PacketType = PacketType::SMeterCommand;

    fn write_payload(&self, payload: &mut [u8]) {
        let bytes = self.value.to_be_bytes();
        payload[0] = bytes[0];
        payload[1] = bytes[1];
    }

    fn read_payload(payload: &[u8]) -> Option<Self> {
        Some(Self {
            value: u16::from_be_bytes([payload[0], payload[1]]),
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

impl PacketSerializable for Wm8940Command {
    const PACKET_TYPE: PacketType = PacketType::Wm8940Command;

    fn write_payload(&self, payload: &mut [u8]) {
        payload[0] = self.dac_volume_left;
        payload[1] = self.dac_volume_right;
        payload[2] = self.adc_volume_left;
        payload[3] = self.adc_volume_right;
        payload[4] = if self.enable { 1 } else { 0 };
    }

    fn read_payload(payload: &[u8]) -> Option<Self> {
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

impl PacketSerializable for DisplayCommand {
    const PACKET_TYPE: PacketType = PacketType::DisplayCommand;

    fn write_payload(&self, payload: &mut [u8]) {
        payload[0] = self.display_id;
        let data_len = DISPLAY_BUFFER_SIZE.min(payload.len() - 3);
        let len_bytes = (data_len as u16).to_be_bytes();
        payload[1] = len_bytes[0];
        payload[2] = len_bytes[1];
        payload[3..3 + data_len].copy_from_slice(&self.buffer[..data_len]);
    }

    fn read_payload(payload: &[u8]) -> Option<Self> {
        let display_id = payload[0];
        let data_len = u16::from_be_bytes([payload[1], payload[2]]) as usize;
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

impl PacketSerializable for DisplayEnableCommand {
    const PACKET_TYPE: PacketType = PacketType::DisplayEnableCommand;

    fn write_payload(&self, payload: &mut [u8]) {
        payload[0] = if self.enabled { 1 } else { 0 };
    }

    fn read_payload(payload: &[u8]) -> Option<Self> {
        Some(Self {
            enabled: payload[0] != 0,
        })
    }
}
