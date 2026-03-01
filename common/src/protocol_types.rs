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
pub struct RadioStateCommand {
    pub rssi_dbm: i8,
    pub forward_power_mw: u16,
    pub vswr_x100: u16,
    pub mode: Mode,
    pub transmit_mode: TransmitMode,
    pub agc_mode: IfGainMode,
    pub rf_gain_mode: RfGainMode,
    pub filter_bw_hz: u16,
    pub frequency: u32,
    pub band: u8,
    pub nb_enabled: bool,
    pub clarifier_mode: u8,
    pub clarifier_raw: i16,
    pub rf_power_centipercent: u16,
    pub volume_raw: i16,
    pub squelch_raw: i16,
    pub cursor_index: u8,
    pub cursor_editing: bool,
}

impl PacketSerializable for RadioStateCommand {
    const PACKET_TYPE: PacketType = PacketType::RadioStateCommand;

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
        let freq = self.frequency.to_be_bytes();
        payload[11] = freq[0];
        payload[12] = freq[1];
        payload[13] = freq[2];
        payload[14] = freq[3];
        payload[15] = self.band;
        payload[16] = if self.nb_enabled { 1 } else { 0 };
        payload[17] = self.clarifier_mode;
        let cr = self.clarifier_raw.to_be_bytes();
        payload[18] = cr[0];
        payload[19] = cr[1];
        let rp = self.rf_power_centipercent.to_be_bytes();
        payload[20] = rp[0];
        payload[21] = rp[1];
        let vol = self.volume_raw.to_be_bytes();
        payload[22] = vol[0];
        payload[23] = vol[1];
        let sql = self.squelch_raw.to_be_bytes();
        payload[24] = sql[0];
        payload[25] = sql[1];
        payload[26] = self.cursor_index;
        payload[27] = if self.cursor_editing { 1 } else { 0 };
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
            frequency: u32::from_be_bytes([payload[11], payload[12], payload[13], payload[14]]),
            band: payload[15],
            nb_enabled: payload[16] != 0,
            clarifier_mode: payload[17],
            clarifier_raw: i16::from_be_bytes([payload[18], payload[19]]),
            rf_power_centipercent: u16::from_be_bytes([payload[20], payload[21]]),
            volume_raw: i16::from_be_bytes([payload[22], payload[23]]),
            squelch_raw: i16::from_be_bytes([payload[24], payload[25]]),
            cursor_index: payload[26],
            cursor_editing: payload[27] != 0,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MenuCommand {
    Ok,
    Cancel,
    Scroll(i8),
}

impl PacketSerializable for MenuCommand {
    const PACKET_TYPE: PacketType = PacketType::MenuCommand;

    fn write_payload(&self, payload: &mut [u8]) {
        match self {
            MenuCommand::Ok => {
                payload[0] = 0;
                payload[1] = 0;
            }
            MenuCommand::Cancel => {
                payload[0] = 1;
                payload[1] = 0;
            }
            MenuCommand::Scroll(delta) => {
                payload[0] = 2;
                payload[1] = *delta as u8;
            }
        }
    }

    fn read_payload(payload: &[u8]) -> Option<Self> {
        match payload[0] {
            0 => Some(MenuCommand::Ok),
            1 => Some(MenuCommand::Cancel),
            2 => Some(MenuCommand::Scroll(payload[1] as i8)),
            _ => None,
        }
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

#[derive(Debug, Clone, Copy)]
pub struct DisplayFpsEvent {
    pub fps: [u16; 3],
}

impl PacketSerializable for DisplayFpsEvent {
    const PACKET_TYPE: PacketType = PacketType::DisplayFpsEvent;

    fn write_payload(&self, payload: &mut [u8]) {
        let f0 = self.fps[0].to_be_bytes();
        payload[0] = f0[0];
        payload[1] = f0[1];
        let f1 = self.fps[1].to_be_bytes();
        payload[2] = f1[0];
        payload[3] = f1[1];
        let f2 = self.fps[2].to_be_bytes();
        payload[4] = f2[0];
        payload[5] = f2[1];
    }

    fn read_payload(payload: &[u8]) -> Option<Self> {
        Some(Self {
            fps: [
                u16::from_be_bytes([payload[0], payload[1]]),
                u16::from_be_bytes([payload[2], payload[3]]),
                u16::from_be_bytes([payload[4], payload[5]]),
            ],
        })
    }
}

#[derive(Clone, Copy)]
pub struct CrashInfoCommand {
    pub reset_reason: u8,
    pub pc: u32,
    pub lr: u32,
    pub panic_line: u32,
    pub panic_file: [u8; 64],
    pub uptime_secs: u32,
}

impl PacketSerializable for CrashInfoCommand {
    const PACKET_TYPE: PacketType = PacketType::CrashInfoCommand;

    fn write_payload(&self, payload: &mut [u8]) {
        payload[0] = self.reset_reason;
        let pc = self.pc.to_be_bytes();
        payload[1] = pc[0];
        payload[2] = pc[1];
        payload[3] = pc[2];
        payload[4] = pc[3];
        let lr = self.lr.to_be_bytes();
        payload[5] = lr[0];
        payload[6] = lr[1];
        payload[7] = lr[2];
        payload[8] = lr[3];
        let pl = self.panic_line.to_be_bytes();
        payload[9] = pl[0];
        payload[10] = pl[1];
        payload[11] = pl[2];
        payload[12] = pl[3];
        payload[13..77].copy_from_slice(&self.panic_file);
        let up = self.uptime_secs.to_be_bytes();
        payload[77] = up[0];
        payload[78] = up[1];
        payload[79] = up[2];
        payload[80] = up[3];
    }

    fn read_payload(payload: &[u8]) -> Option<Self> {
        if payload.len() < 81 {
            return None;
        }
        let mut panic_file = [0u8; 64];
        panic_file.copy_from_slice(&payload[13..77]);
        Some(Self {
            reset_reason: payload[0],
            pc: u32::from_be_bytes([payload[1], payload[2], payload[3], payload[4]]),
            lr: u32::from_be_bytes([payload[5], payload[6], payload[7], payload[8]]),
            panic_line: u32::from_be_bytes([payload[9], payload[10], payload[11], payload[12]]),
            panic_file,
            uptime_secs: u32::from_be_bytes([payload[77], payload[78], payload[79], payload[80]]),
        })
    }
}
