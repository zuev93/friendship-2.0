use embassy_futures::select::{select, select4, Either, Either4};
use embassy_time::Timer;

use crate::{
    app::events::{
        BAND, CLARIFIER_MODE, CLARIFIER_VALUE, COUPLER_METRICS, FILTER, FREQUENCY, IF_GAIN_MODE,
        MODE, NB_ENABLED, RF_GAIN_MODE, RF_POWER, SQUELCH, TRANSMIT_MODE, VOLUME,
    },
    app::types::{Band, ClarifierMode, IfGainMode, Mode, RfGainMode, TransmitMode},
    front_panel::{tasks::spi_receiver::handle_response_packet, types::ControlBusType},
    main_board::events::CURRENT_RSSI,
};
use common::protocol_types::RadioStateCommand;

#[embassy_executor::task]
pub async fn radio_state_task(control_bus: ControlBusType) {
    let mut rssi_dbm: i8 = -120;
    let mut forward_power_mw: u16 = 0;
    let mut vswr_x100: u16 = 100;
    let mut mode = Mode::StandBy;
    let mut transmit_mode = TransmitMode::Usb;
    let mut agc_mode = IfGainMode::Manual;
    let mut rf_gain_mode = RfGainMode::Normal;
    let mut filter_bw_hz: u16 = 2400;
    let mut frequency: u32 = 7_100_000;
    let mut band = Band::Band40m;
    let mut nb_enabled = false;
    let mut clarifier_mode = ClarifierMode::Off;
    let mut clarifier_raw: i16 = 500;
    let mut rf_power_centipercent: u16 = 5000;
    let mut volume_raw: i16 = 500;
    let mut squelch_raw: i16 = 0;

    let mut rssi_rcv = CURRENT_RSSI.receiver().unwrap();
    let mut coupler_rcv = COUPLER_METRICS.receiver().unwrap();
    let mut mode_rcv = MODE.receiver().unwrap();
    let mut transmit_mode_rcv = TRANSMIT_MODE.receiver().unwrap();
    let mut if_gain_mode_rcv = IF_GAIN_MODE.receiver().unwrap();
    let mut rf_gain_mode_rcv = RF_GAIN_MODE.receiver().unwrap();
    let mut filter_rcv = FILTER.receiver().unwrap();
    let mut frequency_rcv = FREQUENCY.receiver().unwrap();
    let mut band_rcv = BAND.receiver().unwrap();
    let mut nb_enabled_rcv = NB_ENABLED.receiver().unwrap();
    let mut clarifier_mode_rcv = CLARIFIER_MODE.receiver().unwrap();
    let mut clarifier_value_rcv = CLARIFIER_VALUE.receiver().unwrap();
    let mut rf_power_rcv = RF_POWER.receiver().unwrap();
    let mut volume_rcv = VOLUME.receiver().unwrap();
    let mut squelch_rcv = SQUELCH.receiver().unwrap();

    loop {
        match select(
            select4(
                rssi_rcv.changed(),
                coupler_rcv.changed(),
                mode_rcv.changed(),
                transmit_mode_rcv.changed(),
            ),
            select(
                select4(
                    if_gain_mode_rcv.changed(),
                    rf_gain_mode_rcv.changed(),
                    filter_rcv.changed(),
                    frequency_rcv.changed(),
                ),
                select(
                    select4(
                        band_rcv.changed(),
                        nb_enabled_rcv.changed(),
                        clarifier_mode_rcv.changed(),
                        clarifier_value_rcv.changed(),
                    ),
                    select4(
                        rf_power_rcv.changed(),
                        volume_rcv.changed(),
                        squelch_rcv.changed(),
                        Timer::after_millis(16),
                    ),
                ),
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
                Either::First(inner) => match inner {
                    Either4::First(agc) => {
                        agc_mode = agc;
                    }
                    Either4::Second(rf) => {
                        rf_gain_mode = rf;
                    }
                    Either4::Third(f) => {
                        filter_bw_hz = f.bandwidth_hz() as u16;
                    }
                    Either4::Fourth(f) => {
                        frequency = f;
                    }
                },
                Either::Second(inner2) => match inner2 {
                    Either::First(inner3) => match inner3 {
                        Either4::First(b) => {
                            band = b;
                        }
                        Either4::Second(nb) => {
                            nb_enabled = nb;
                        }
                        Either4::Third(cm) => {
                            clarifier_mode = cm;
                        }
                        Either4::Fourth(cv) => {
                            clarifier_raw = cv.raw();
                        }
                    },
                    Either::Second(inner4) => match inner4 {
                        Either4::First(rp) => {
                            rf_power_centipercent = rp.centipercent;
                        }
                        Either4::Second(vol) => {
                            volume_raw = vol.raw();
                        }
                        Either4::Third(sql) => {
                            squelch_raw = sql.raw();
                        }
                        Either4::Fourth(_) => {}
                    },
                },
            },
        }

        let band_u8 = match band {
            Band::Band160m => 0,
            Band::Band80m => 1,
            Band::Band40m => 2,
            Band::Band30m => 3,
            Band::Band20m => 4,
            Band::Band17m => 5,
            Band::Band15m => 6,
            Band::Band12m => 7,
            Band::Band10m => 8,
        };

        let clarifier_mode_u8 = match clarifier_mode {
            ClarifierMode::Off => 0,
            ClarifierMode::Rit => 1,
            ClarifierMode::XIT => 2,
        };

        let cmd = RadioStateCommand {
            rssi_dbm,
            forward_power_mw,
            vswr_x100,
            mode,
            transmit_mode,
            agc_mode,
            rf_gain_mode,
            filter_bw_hz,
            frequency,
            band: band_u8,
            nb_enabled,
            clarifier_mode: clarifier_mode_u8,
            clarifier_raw,
            rf_power_centipercent,
            volume_raw,
            squelch_raw,
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
