use embassy_stm32::peripherals as stm_peripherals;
use embassy_stm32::usb::Driver;
use embassy_usb::class::uac1::speaker::{ControlMonitor, Feedback, Stream};

use crate::app::events::USB_AUDIO_TX;
use crate::consts::AUDIO_BUFFER_SIZE;

type UsbDriver = Driver<'static, stm_peripherals::USB>;

const FEEDBACK_48K: [u8; 3] = [0x00, 0x00, 0x0C];

#[embassy_executor::task]
pub async fn speaker_task(
    mut stream: Stream<'static, UsbDriver>,
    mut feedback: Feedback<'static, UsbDriver>,
    _control_monitor: ControlMonitor<'static>,
) {
    let mut audio_buf = [32768u16; AUDIO_BUFFER_SIZE];
    let mut packet_buf = [0u8; 192];

    loop {
        stream.wait_connection().await;
        let mut audio_pos: usize = 0;

        loop {
            let n = match stream.read_packet(&mut packet_buf).await {
                Ok(n) => n,
                Err(_) => break,
            };

            let samples = n / 2;
            for i in 0..samples {
                let lo = packet_buf[i * 2] as i16;
                let hi = packet_buf[i * 2 + 1] as i16;
                let signed = lo | (hi << 8);
                let unsigned = (signed as i32 + 32768) as u16;

                audio_buf[audio_pos] = unsigned;
                audio_pos += 1;

                if audio_pos >= AUDIO_BUFFER_SIZE {
                    USB_AUDIO_TX.sender().send(audio_buf);
                    audio_buf = [32768u16; AUDIO_BUFFER_SIZE];
                    audio_pos = 0;
                }
            }

            let _ = feedback.write_packet(&FEEDBACK_48K).await;
        }
    }
}
