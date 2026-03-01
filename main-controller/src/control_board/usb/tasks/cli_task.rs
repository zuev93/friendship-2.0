use embassy_stm32::peripherals as stm_peripherals;
use embassy_stm32::usb::Driver;
use embassy_usb::class::cdc_acm::CdcAcmClass;

use crate::app::events::{CPU_PERCENT, DISPLAY_FPS, RAM_STATS};
use crate::runtime_stats::TaskId;
use druzhba_macros::instrumented;

type UsbDriver = Driver<'static, stm_peripherals::USB>;

#[instrumented(TaskId::CliTask)]
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
    } else if cmd == b"stats" {
        write_stats(cdc).await
    } else if cmd == b"help" {
        cdc.write_packet(b"Commands: version, status, stats, echo <text>, help\r\n")
            .await
    } else {
        cdc.write_packet(b"?\r\n").await
    }
}

async fn write_stats(
    cdc: &mut CdcAcmClass<'static, UsbDriver>,
) -> Result<(), embassy_usb::driver::EndpointError> {
    let mut line = [0u8; 64];

    let cpu = CPU_PERCENT.try_get().map(|c| c.raw()).unwrap_or(0);
    let len = write_u32_line(&mut line, b"cpu%: ", cpu as u32);
    cdc.write_packet(&line[..len]).await?;

    if let Some(ram) = RAM_STATS.try_get() {
        let len = write_u32_line(&mut line, b"ram_static: ", ram.static_used as u32);
        cdc.write_packet(&line[..len]).await?;
        let len = write_u32_line(&mut line, b"ram_total: ", ram.total_ram as u32);
        cdc.write_packet(&line[..len]).await?;
    }

    if let Some(fps) = DISPLAY_FPS.try_get() {
        let f = fps.raw();
        let len = write_u32_line(&mut line, b"fps_meter: ", f[0] as u32);
        cdc.write_packet(&line[..len]).await?;
        let len = write_u32_line(&mut line, b"fps_spectrum: ", f[1] as u32);
        cdc.write_packet(&line[..len]).await?;
        let len = write_u32_line(&mut line, b"fps_main: ", f[2] as u32);
        cdc.write_packet(&line[..len]).await?;
    }

    Ok(())
}

fn write_u32_line(buf: &mut [u8], prefix: &[u8], val: u32) -> usize {
    let mut pos = 0;
    buf[..prefix.len()].copy_from_slice(prefix);
    pos += prefix.len();
    if val == 0 {
        buf[pos] = b'0';
        pos += 1;
    } else {
        let mut digits = [0u8; 10];
        let mut n = val;
        let mut count = 0;
        while n > 0 {
            digits[count] = b'0' + (n % 10) as u8;
            n /= 10;
            count += 1;
        }
        for i in (0..count).rev() {
            buf[pos] = digits[i];
            pos += 1;
        }
    }
    buf[pos] = b'\r';
    buf[pos + 1] = b'\n';
    pos + 2
}
