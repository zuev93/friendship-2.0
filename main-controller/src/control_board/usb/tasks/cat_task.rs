use embassy_stm32::peripherals as stm_peripherals;
use embassy_stm32::usb::Driver;
use embassy_usb::class::cdc_acm::CdcAcmClass;

use crate::control_board::usb::cat::handler::CatHandler;
use crate::control_board::usb::cat::parser::CatParser;
use crate::runtime_stats::TaskId;
use druzhba_macros::instrumented;

type UsbDriver = Driver<'static, stm_peripherals::USB>;

#[instrumented(TaskId::CatTask)]
#[embassy_executor::task]
pub async fn cat_task(mut cdc: CdcAcmClass<'static, UsbDriver>) {
    let mut parser = CatParser::new();
    let mut handler = CatHandler::new();
    let mut response_buf = [0u8; 64];

    loop {
        cdc.wait_connection().await;
        parser.reset();

        loop {
            let mut read_buf = [0u8; 64];
            let n = match cdc.read_packet(&mut read_buf).await {
                Ok(n) => n,
                Err(_) => break,
            };

            for i in 0..n {
                if let Some(cmd) = parser.feed(read_buf[i]) {
                    let response = handler.handle(cmd);
                    let len = response.format(&mut response_buf);
                    if len > 0 {
                        if cdc.write_packet(&response_buf[..len]).await.is_err() {
                            break;
                        }
                    }
                }
            }
        }
    }
}
