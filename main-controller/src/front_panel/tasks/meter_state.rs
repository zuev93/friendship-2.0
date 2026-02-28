use embassy_futures::select::{select, select4, Either, Either4};
use embassy_time::Timer;

use crate::{
    app::events::{
        COUPLER_METRICS, CURRENT_FILTER, CURRENT_IF_GAIN_MODE, CURRENT_MODE,
        CURRENT_RF_GAIN_MODE, CURRENT_TRANSMIT_MODE,
    },
    app::types::{IfGainMode, Mode, RfGainMode, TransmitMode},
    front_panel::{tasks::spi_receiver::handle_response_packet, types::ControlBusType},
    main_board::events::CURRENT_RSSI,
};
use common::protocol_types::MeterStateCommand;

#[embassy_executor::task]
pub async fn meter_state_task(control_bus: ControlBusType) {
    let mut rssi_dbm: i8 = -120;
    let mut forward_power_mw: u16 = 0;
    let mut vswr_x100: u16 = 100;
    let mut mode = Mode::StandBy;
    let mut transmit_mode = TransmitMode::Usb;
    let mut agc_mode = IfGainMode::Manual;
    let mut rf_gain_mode = RfGainMode::Normal;
    let mut filter_bw_hz: u16 = 2400;

    loop {
        match select(
            select4(
                CURRENT_RSSI.wait(),
                COUPLER_METRICS.wait(),
                CURRENT_MODE.wait(),
                CURRENT_TRANSMIT_MODE.wait(),
            ),
            select4(
                CURRENT_IF_GAIN_MODE.wait(),
                CURRENT_RF_GAIN_MODE.wait(),
                CURRENT_FILTER.wait(),
                Timer::after_millis(16),
            ),
        )
        .await
        {
            Either::First(first) => match first {
                Either4::First(rssi) => {
                    rssi_dbm = rssi.dbm;
                }
                Either4::Second(metrics) => {
                    forward_power_mw = (metrics.forward_w * 1000.0) as u16;
                    vswr_x100 = (metrics.vswr * 100.0) as u16;
                }
                Either4::Third(m) => {
                    mode = m;
                }
                Either4::Fourth(tm) => {
                    transmit_mode = tm;
                }
            },
            Either::Second(second) => match second {
                Either4::First(agc) => {
                    agc_mode = agc;
                }
                Either4::Second(rf) => {
                    rf_gain_mode = rf;
                }
                Either4::Third(f) => {
                    filter_bw_hz = f.bandwidth_hz() as u16;
                }
                Either4::Fourth(_) => {}
            },
        }

        let cmd = MeterStateCommand {
            rssi_dbm,
            forward_power_mw,
            vswr_x100,
            mode,
            transmit_mode,
            agc_mode,
            rf_gain_mode,
            filter_bw_hz,
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
