use crate::app::events::{CURRENT_FREQUENCY, WATERFALL_LINE};
use crate::front_panel::{tasks::spi_receiver::handle_response_packet, types::ControlBusType};
use common::protocol_types::{SweepStatus, WaterfallLineCommand};

const DEFAULT_SPAN_HZ: u32 = 100_000;

#[embassy_executor::task]
pub async fn waterfall_sender_task(control_bus: ControlBusType) {
    let mut vfo_freq: u32 = 7_100_000;

    loop {
        let line = WATERFALL_LINE.wait().await;

        if let Some(freq) = CURRENT_FREQUENCY.try_take() {
            vfo_freq = freq;
        }

        let cmd = WaterfallLineCommand {
            center_freq: vfo_freq,
            span_hz: DEFAULT_SPAN_HZ,
            sweep_status: SweepStatus::Sweeping,
            bins: line.bins,
        };

        let response = {
            let mut spi = control_bus.lock().await;
            spi.send(&cmd).await
        };

        if let Ok(response_packet) = response {
            handle_response_packet(&response_packet).await;
        }
    }
}
