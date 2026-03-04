use embassy_futures::select::{select, select3, Either, Either3};

use crate::{
    app::events::{ALC_CONSTRAINT, MODE, RF_POWER, TX_THERMAL_CONSTRAINT},
    control_board::events::{PD_CONTRACT, POWER_TELEMETRY},
    main_board::modules::detector::Detector,
    runtime_stats::TaskId,
};
use common::error::error;
use druzhba_macros::instrumented;

#[instrumented(TaskId::DetectorTasks)]
#[embassy_executor::task]
pub async fn detector_tasks(mut detector: Detector) {
    let mut mode_rcv = MODE.receiver().unwrap();
    let mut rf_power_rcv = RF_POWER.receiver().unwrap();
    let mut power_telemetry_rcv = POWER_TELEMETRY.receiver().unwrap();
    let mut pd_contract_rcv = PD_CONTRACT.receiver().unwrap();
    let mut tx_thermal_rcv = TX_THERMAL_CONSTRAINT.receiver().unwrap();
    let mut alc_rcv = ALC_CONSTRAINT.receiver().unwrap();
    loop {
        match select(
            select3(
                mode_rcv.changed(),
                rf_power_rcv.changed(),
                power_telemetry_rcv.changed(),
            ),
            select3(
                pd_contract_rcv.changed(),
                tx_thermal_rcv.changed(),
                alc_rcv.changed(),
            ),
        )
        .await
        {
            Either::First(inner) => match inner {
                Either3::First(mode) => {
                    if let Err(e) = detector.set_mode(mode).await {
                        error(e).await;
                    }
                }
                Either3::Second(rf_power) => {
                    if let Err(e) = detector.set_power(rf_power).await {
                        error(e).await;
                    }
                }
                Either3::Third(telemetry) => {
                    if let Err(e) = detector.set_power_telemetry(telemetry).await {
                        error(e).await;
                    }
                }
            },
            Either::Second(inner) => match inner {
                Either3::First(contract) => {
                    if let Err(e) = detector.set_pd_contract(contract).await {
                        error(e).await;
                    }
                }
                Either3::Second(thermal) => {
                    if let Err(e) = detector.set_thermal_constraint(thermal).await {
                        error(e).await;
                    }
                }
                Either3::Third(alc) => {
                    if let Err(e) = detector.set_alc_constraint(alc).await {
                        error(e).await;
                    }
                }
            },
        }
    }
}
