use embassy_executor::Spawner;
use embassy_futures::select::{select4, Either4};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use embassy_time::{Duration, Ticker};
use static_cell::StaticCell;

use crate::{
    app::events::MODE,
    control_board::events::{
        EmergencyReason, PdContract, EMERGENCY_SHUTDOWN, PA_CURRENT_READING, PA_CURRENT_REQUEST,
        PA_FAST_MODE, PD_CONTRACT, POWER_TELEMETRY,
    },
    control_board::modules::power_control::PowerControl,
};
use common::error::error;

const TELEMETRY_PERIOD: Duration = Duration::from_millis(10);

pub fn create_tasks(spawner: Spawner, power_control: PowerControl) {
    static POWER_CONTROL: StaticCell<Mutex<ThreadModeRawMutex, PowerControl>> = StaticCell::new();
    let mutex = POWER_CONTROL.init(Mutex::new(power_control));

    spawner.must_spawn(power_control_task(mutex));
    spawner.must_spawn(power_monitor_task(mutex));
}

#[embassy_executor::task]
async fn power_control_task(mutex: &'static Mutex<ThreadModeRawMutex, PowerControl>) {
    let mut mode_rcv = MODE.receiver().unwrap();
    loop {
        let mode = mode_rcv.changed().await;
        if let Err(e) = mutex.lock().await.set_mode(mode).await {
            error(e).await;
        }
    }
}

#[embassy_executor::task]
async fn power_monitor_task(mutex: &'static Mutex<ThreadModeRawMutex, PowerControl>) {
    let mut ticker = Ticker::every(TELEMETRY_PERIOD);
    let mut last_contract = PdContract::default();
    let mut pa_current_req_rcv = PA_CURRENT_REQUEST.receiver().unwrap();
    let mut pa_fast_mode_rcv = PA_FAST_MODE.receiver().unwrap();
    let mut pd_contract_rcv = PD_CONTRACT.receiver().unwrap();

    loop {
        match select4(
            ticker.next(),
            pa_current_req_rcv.changed(),
            pa_fast_mode_rcv.changed(),
            pd_contract_rcv.changed(),
        )
        .await
        {
            Either4::First(_) => {
                let mut pc = mutex.lock().await;
                match pc.read_power_telemetry().await {
                    Ok(telemetry) => {
                        if let Some(reason) = pc.check_limits(&telemetry, &last_contract) {
                            pc.emergency_shutdown();
                            EMERGENCY_SHUTDOWN.sender().send(reason);
                            match reason {
                                EmergencyReason::VbusOvercurrent => {
                                    error("Emergency: VBUS overcurrent").await;
                                }
                                EmergencyReason::PaOvercurrent => {
                                    error("Emergency: PA overcurrent outside TX").await;
                                }
                                EmergencyReason::Thermal => {
                                    error("Emergency: thermal shutdown").await;
                                }
                            }
                        }
                        POWER_TELEMETRY.sender().send(telemetry);
                    }
                    Err(e) => error(e).await,
                }
            }
            Either4::Second(_) => {
                let mut pc = mutex.lock().await;
                match pc.read_pa_current_ma().await {
                    Ok(current) => PA_CURRENT_READING.sender().send(current),
                    Err(e) => {
                        error(e).await;
                        PA_CURRENT_READING.sender().send(0);
                    }
                }
            }
            Either4::Third(fast) => {
                let mut pc = mutex.lock().await;
                let result = if fast {
                    pc.set_pa_fast_mode().await
                } else {
                    pc.set_pa_normal_mode().await
                };
                if let Err(e) = result {
                    error(e).await;
                }
            }
            Either4::Fourth(contract) => {
                last_contract = contract;
            }
        }
    }
}
