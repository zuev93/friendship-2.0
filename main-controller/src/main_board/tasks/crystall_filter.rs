use embassy_futures::select::{select, select6, Either, Either6};

use crate::{
    app::events::{
        CURRENT_FILTER, CURRENT_MODE, CURRENT_RF_GAIN_MODE, CURRENT_RF_POWER,
        TX_THERMAL_CONSTRAINT,
    },
    control_board::events::{PD_CONTRACT, POWER_TELEMETRY},
    main_board::modules::crystall_filter::CrystallFilter,
};
use common::error::error;

#[embassy_executor::task]
pub async fn crystall_filter_task(mut crystall_filter: CrystallFilter) {
    loop {
        match select(
            select6(
                CURRENT_RF_POWER.wait(),
                POWER_TELEMETRY.wait(),
                PD_CONTRACT.wait(),
                TX_THERMAL_CONSTRAINT.wait(),
                CURRENT_MODE.wait(),
                CURRENT_FILTER.wait(),
            ),
            CURRENT_RF_GAIN_MODE.wait(),
        )
        .await
        {
            Either::First(inner) => match inner {
                Either6::First(rf_power) => {
                    if let Err(e) = crystall_filter.set_power(rf_power).await {
                        error(e).await;
                    }
                }
                Either6::Second(telemetry) => {
                    if let Err(e) = crystall_filter.set_power_telemetry(telemetry).await {
                        error(e).await;
                    }
                }
                Either6::Third(contract) => {
                    if let Err(e) = crystall_filter.set_pd_contract(contract).await {
                        error(e).await;
                    }
                }
                Either6::Fourth(thermal) => {
                    if let Err(e) = crystall_filter.set_thermal_constraint(thermal).await {
                        error(e).await;
                    }
                }
                Either6::Fifth(mode) => {
                    if let Err(e) = crystall_filter.set_mode(mode).await {
                        error(e).await;
                    }
                }
                Either6::Sixth(filter) => {
                    if let Err(e) = crystall_filter.set_filter_type(filter).await {
                        error(e).await;
                    }
                }
            },
            Either::Second(rf_gain_mode) => {
                if let Err(e) = crystall_filter.set_rf_gain_mode(rf_gain_mode).await {
                    error(e).await;
                }
            }
        }
    }
}
