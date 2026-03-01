use crate::app::events::{FREQUENCY, WATERFALL_LINE};
use crate::front_panel::{tasks::spi_receiver::handle_response_packet, types::ControlBusType};
use crate::runtime_stats::TaskId;
use common::protocol_types::{SweepStatus, WaterfallLineCommand};
use druzhba_macros::instrumented;

const DEFAULT_SPAN_HZ: u32 = 100_000;

#[instrumented(TaskId::WaterfallSender)]
#[embassy_executor::task]
pub async fn waterfall_sender_task(control_bus: ControlBusType) {
    let mut vfo_freq: u32 = 7_100_000;
    let mut waterfall_rcv = WATERFALL_LINE.receiver().unwrap();
    let mut frequency_rcv = FREQUENCY.anon_receiver();

    loop {
        let line = waterfall_rcv.changed().await;

        if let Some(freq) = frequency_rcv.try_changed() {
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
