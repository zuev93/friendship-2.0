use crate::app::types::{Frequency, TransmitMode};

pub enum CatCommand {
    Id,
    FrequencyRead,
    FrequencyWrite(Frequency),
    ModeRead,
    ModeWrite(TransmitMode),
    InfoRead,
    Transmit,
    Receive,
}

pub struct CatParser {
    buf: [u8; 64],
    len: usize,
}

impl CatParser {
    pub fn new() -> Self {
        Self {
            buf: [0; 64],
            len: 0,
        }
    }

    pub fn reset(&mut self) {
        self.len = 0;
    }

    pub fn feed(&mut self, byte: u8) -> Option<CatCommand> {
        if byte == b';' {
            let result = self.parse();
            self.len = 0;
            return result;
        }

        if self.len < self.buf.len() {
            self.buf[self.len] = byte;
            self.len += 1;
        }

        None
    }

    fn parse(&self) -> Option<CatCommand> {
        if self.len < 2 {
            return None;
        }

        let cmd = [self.buf[0], self.buf[1]];
        let args = &self.buf[2..self.len];

        match &cmd {
            b"ID" => Some(CatCommand::Id),
            b"FA" => {
                if args.is_empty() {
                    Some(CatCommand::FrequencyRead)
                } else {
                    parse_frequency(args).map(CatCommand::FrequencyWrite)
                }
            }
            b"MD" => {
                if args.is_empty() {
                    Some(CatCommand::ModeRead)
                } else {
                    parse_mode(args).map(CatCommand::ModeWrite)
                }
            }
            b"IF" => Some(CatCommand::InfoRead),
            b"TX" => Some(CatCommand::Transmit),
            b"RX" => Some(CatCommand::Receive),
            _ => None,
        }
    }
}

fn parse_frequency(args: &[u8]) -> Option<Frequency> {
    let mut freq: u32 = 0;
    for &b in args {
        if b < b'0' || b > b'9' {
            return None;
        }
        freq = freq.checked_mul(10)?.checked_add((b - b'0') as u32)?;
    }
    Some(freq)
}

fn parse_mode(args: &[u8]) -> Option<TransmitMode> {
    if args.len() != 1 {
        return None;
    }
    match args[0] {
        b'1' => Some(TransmitMode::Lsb),
        b'2' => Some(TransmitMode::Usb),
        b'3' => Some(TransmitMode::Cw),
        b'5' => Some(TransmitMode::Am),
        _ => None,
    }
}
