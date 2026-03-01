use embassy_stm32::peripherals as stm_peripherals;
use embassy_stm32::usb::Driver;
use embassy_usb::class::cdc_acm::CdcAcmClass;

type UsbDriver = Driver<'static, stm_peripherals::USB>;

#[embassy_executor::task]
pub async fn cli_task(mut cdc: CdcAcmClass<'static, UsbDriver>) {
    let mut buf = [0u8; 128];

    loop {
        cdc.wait_connection().await;
        let mut pos: usize = 0;

        loop {
            let mut read_buf = [0u8; 64];
            let n = match cdc.read_packet(&mut read_buf).await {
                Ok(n) => n,
                Err(_) => break,
            };

            for i in 0..n {
                let byte = read_buf[i];
                if byte == b'\n' || byte == b'\r' {
                    if pos > 0 {
                        if write_response(&mut cdc, &buf[..pos]).await.is_err() {
                            pos = 0;
                            break;
                        }
                        pos = 0;
                    }
                } else if pos < buf.len() {
                    buf[pos] = byte;
                    pos += 1;
                }
            }
        }
    }
}

async fn write_response(
    cdc: &mut CdcAcmClass<'static, UsbDriver>,
    cmd: &[u8],
) -> Result<(), embassy_usb::driver::EndpointError> {
    if cmd == b"version" {
        cdc.write_packet(b"Druzhba-M v0.1.0\r\n").await
    } else if cmd == b"status" {
        cdc.write_packet(b"status: ok\r\n").await
    } else if cmd.starts_with(b"echo ") {
        let text = &cmd[5..];
        cdc.write_packet(text).await?;
        cdc.write_packet(b"\r\n").await
    } else if cmd == b"help" {
        cdc.write_packet(b"Commands: version, status, echo <text>, help\r\n")
            .await
    } else {
        cdc.write_packet(b"?\r\n").await
    }
}
