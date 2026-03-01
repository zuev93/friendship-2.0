use crate::app::types::{Frequency, TransmitMode};

pub enum CatResponse {
    Id,
    Frequency(Frequency),
    ModeValue(TransmitMode),
    Info {
        frequency: Frequency,
        mode: TransmitMode,
        transmitting: bool,
    },
    Tx,
    Rx,
}

impl CatResponse {
    pub fn format(&self, buf: &mut [u8]) -> usize {
        match self {
            CatResponse::Id => copy_to_buf(buf, b"ID023;"),
            CatResponse::Frequency(freq) => {
                let mut pos = copy_to_buf(buf, b"FA");
                pos += format_frequency(*freq, &mut buf[pos..]);
                buf[pos] = b';';
                pos + 1
            }
            CatResponse::ModeValue(mode) => {
                copy_to_buf(buf, &[b'M', b'D', mode_to_byte(*mode), b';'])
            }
            CatResponse::Info {
                frequency,
                mode,
                transmitting,
            } => format_if_response(buf, *frequency, *mode, *transmitting),
            CatResponse::Tx => copy_to_buf(buf, b"TX0;"),
            CatResponse::Rx => copy_to_buf(buf, b"RX0;"),
        }
    }
}

fn copy_to_buf(buf: &mut [u8], data: &[u8]) -> usize {
    let len = data.len().min(buf.len());
    buf[..len].copy_from_slice(&data[..len]);
    len
}

fn format_frequency(freq: Frequency, buf: &mut [u8]) -> usize {
    if buf.len() < 11 {
        return 0;
    }
    let mut val = freq;
    let mut i = 10;
    loop {
        buf[i] = b'0' + (val % 10) as u8;
        val /= 10;
        if i == 0 {
            break;
        }
        i -= 1;
    }
    11
}

fn mode_to_byte(mode: TransmitMode) -> u8 {
    match mode {
        TransmitMode::Lsb => b'1',
        TransmitMode::Usb => b'2',
        TransmitMode::Cw => b'3',
        TransmitMode::Am => b'5',
    }
}

fn format_if_response(
    buf: &mut [u8],
    frequency: Frequency,
    mode: TransmitMode,
    transmitting: bool,
) -> usize {
    if buf.len() < 38 {
        return 0;
    }

    buf[0] = b'I';
    buf[1] = b'F';
    format_frequency(frequency, &mut buf[2..13]);
    buf[13] = b' ';
    buf[14] = b' ';
    buf[15] = b' ';
    buf[16] = b' ';
    buf[17] = b' ';
    buf[18] = b'+';
    buf[19] = b'0';
    buf[20] = b'0';
    buf[21] = b'0';
    buf[22] = b'0';
    buf[23] = b'0';
    buf[24] = b'0';
    buf[25] = b'0';
    buf[26] = b'0';
    buf[27] = b'0';
    buf[28] = b'0';
    buf[29] = if transmitting { b'1' } else { b'0' };
    buf[30] = mode_to_byte(mode);
    buf[31] = b'0';
    buf[32] = b'0';
    buf[33] = b'0';
    buf[34] = b'0';
    buf[35] = b'0';
    buf[36] = b'0';
    buf[37] = b';';
    38
}
