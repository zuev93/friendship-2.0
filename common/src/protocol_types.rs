use crate::spi_protocol::{PacketSerializable, PacketType, PACKET_SIZE};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    StandBy,
    WarmUp,
    Rx,
    Tx,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransmitMode {
    Usb,
    Lsb,
    Cw,
    Am,
}

impl TransmitMode {
    pub fn next(self) -> Self {
        match self {
            Self::Usb => Self::Lsb,
            Self::Lsb => Self::Cw,
            Self::Cw => Self::Am,
            Self::Am => Self::Usb,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IfGainMode {
    Manual,
    AgcFast,
    AgcSlow,
}

impl IfGainMode {
    pub fn toggle(self) -> Self {
        match self {
            Self::Manual => Self::AgcFast,
            Self::AgcFast => Self::AgcSlow,
            Self::AgcSlow => Self::Manual,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RfGainMode {
    Attenuator,
    Normal,
    RfSingle,
    RfDouble,
}

impl RfGainMode {
    pub fn next(self) -> Self {
        match self {
            Self::Attenuator => Self::Normal,
            Self::Normal => Self::RfSingle,
            Self::RfSingle => Self::RfDouble,
            Self::RfDouble => Self::Attenuator,
        }
    }
}

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

#[derive(Debug, Clone, Copy)]
pub struct MeterStateCommand {
    pub rssi_dbm: i8,
    pub forward_power_mw: u16,
    pub vswr_x100: u16,
    pub mode: Mode,
    pub transmit_mode: TransmitMode,
    pub agc_mode: IfGainMode,
    pub rf_gain_mode: RfGainMode,
    pub filter_bw_hz: u16,
}

impl PacketSerializable for MeterStateCommand {
    const PACKET_TYPE: PacketType = PacketType::MeterStateCommand;

    fn write_payload(&self, payload: &mut [u8]) {
        payload[0] = self.rssi_dbm as u8;
        let f = self.forward_power_mw.to_be_bytes();
        payload[1] = f[0];
        payload[2] = f[1];
        let v = self.vswr_x100.to_be_bytes();
        payload[3] = v[0];
        payload[4] = v[1];
        payload[5] = match self.mode {
            Mode::StandBy => 0,
            Mode::WarmUp => 1,
            Mode::Rx => 2,
            Mode::Tx => 3,
        };
        payload[6] = match self.transmit_mode {
            TransmitMode::Usb => 0,
            TransmitMode::Lsb => 1,
            TransmitMode::Cw => 2,
            TransmitMode::Am => 3,
        };
        payload[7] = match self.agc_mode {
            IfGainMode::Manual => 0,
            IfGainMode::AgcFast => 1,
            IfGainMode::AgcSlow => 2,
        };
        payload[8] = match self.rf_gain_mode {
            RfGainMode::Attenuator => 0,
            RfGainMode::Normal => 1,
            RfGainMode::RfSingle => 2,
            RfGainMode::RfDouble => 3,
        };
        let bw = self.filter_bw_hz.to_be_bytes();
        payload[9] = bw[0];
        payload[10] = bw[1];
    }

    fn read_payload(payload: &[u8]) -> Option<Self> {
        let mode = match payload[5] {
            0 => Mode::StandBy,
            1 => Mode::WarmUp,
            2 => Mode::Rx,
            3 => Mode::Tx,
            _ => return None,
        };
        let transmit_mode = match payload[6] {
            0 => TransmitMode::Usb,
            1 => TransmitMode::Lsb,
            2 => TransmitMode::Cw,
            3 => TransmitMode::Am,
            _ => return None,
        };
        let agc_mode = match payload[7] {
            0 => IfGainMode::Manual,
            1 => IfGainMode::AgcFast,
            2 => IfGainMode::AgcSlow,
            _ => return None,
        };
        let rf_gain_mode = match payload[8] {
            0 => RfGainMode::Attenuator,
            1 => RfGainMode::Normal,
            2 => RfGainMode::RfSingle,
            3 => RfGainMode::RfDouble,
            _ => return None,
        };
        Some(Self {
            rssi_dbm: payload[0] as i8,
            forward_power_mw: u16::from_be_bytes([payload[1], payload[2]]),
            vswr_x100: u16::from_be_bytes([payload[3], payload[4]]),
            mode,
            transmit_mode,
            agc_mode,
            rf_gain_mode,
            filter_bw_hz: u16::from_be_bytes([payload[9], payload[10]]),
        })
    }
}

pub const WATERFALL_BINS: usize = 240;
const WATERFALL_CMD_SIZE: usize = 4 + 4 + 1 + WATERFALL_BINS;
const _: () = assert!(WATERFALL_CMD_SIZE <= PACKET_SIZE - 3);

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum SweepStatus {
    Idle = 0,
    Sweeping = 1,
    Listening = 2,
}

impl SweepStatus {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Sweeping,
            2 => Self::Listening,
            _ => Self::Idle,
        }
    }
}

#[derive(Clone, Copy)]
pub struct WaterfallLineCommand {
    pub center_freq: u32,
    pub span_hz: u32,
    pub sweep_status: SweepStatus,
    pub bins: [i8; WATERFALL_BINS],
}

impl PacketSerializable for WaterfallLineCommand {
    const PACKET_TYPE: PacketType = PacketType::WaterfallLineCommand;

    fn write_payload(&self, payload: &mut [u8]) {
        let cf = self.center_freq.to_be_bytes();
        payload[0] = cf[0];
        payload[1] = cf[1];
        payload[2] = cf[2];
        payload[3] = cf[3];
        let sp = self.span_hz.to_be_bytes();
        payload[4] = sp[0];
        payload[5] = sp[1];
        payload[6] = sp[2];
        payload[7] = sp[3];
        payload[8] = self.sweep_status as u8;
        let mut i = 0;
        while i < WATERFALL_BINS {
            payload[9 + i] = self.bins[i] as u8;
            i += 1;
        }
    }

    fn read_payload(payload: &[u8]) -> Option<Self> {
        if payload.len() < WATERFALL_CMD_SIZE {
            return None;
        }
        let center_freq = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
        let span_hz = u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]);
        let sweep_status = SweepStatus::from_u8(payload[8]);
        let mut bins = [0i8; WATERFALL_BINS];
        let mut i = 0;
        while i < WATERFALL_BINS {
            bins[i] = payload[9 + i] as i8;
            i += 1;
        }
        Some(Self {
            center_freq,
            span_hz,
            sweep_status,
            bins,
        })
    }
}
