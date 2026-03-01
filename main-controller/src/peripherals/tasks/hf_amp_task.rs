use embassy_executor::Spawner;
use embassy_futures::select::{select, select4, Either, Either4};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use embassy_time::{Duration, Timer};
use static_cell::StaticCell;

use crate::{
    app::events::{MODE, PA_TEMPERATURES, TX_THERMAL_CONSTRAINT},
    control_board::events::{
        EmergencyReason, EMERGENCY_SHUTDOWN, PD_CONTRACT, POWER_TELEMETRY,
    },
    peripherals::modules::hf_amp::HfAmp,
};
use common::error::error;

const THERMAL_SAMPLE_PERIOD: Duration = Duration::from_millis(500);

pub fn create_tasks(spawner: Spawner, hf_amp: HfAmp) {
    static HF_AMP: StaticCell<Mutex<ThreadModeRawMutex, HfAmp>> = StaticCell::new();
    let mutex = HF_AMP.init(Mutex::new(hf_amp));

    spawner.must_spawn(hf_amp_control_task(mutex));
    spawner.must_spawn(hf_amp_thermal_task(mutex));
}

#[embassy_executor::task]
async fn hf_amp_control_task(mutex: &'static Mutex<ThreadModeRawMutex, HfAmp>) {
    let mut mode_rcv = MODE.receiver().unwrap();
    let mut power_telemetry_rcv = POWER_TELEMETRY.receiver().unwrap();
    let mut pd_contract_rcv = PD_CONTRACT.receiver().unwrap();
    let mut tx_thermal_rcv = TX_THERMAL_CONSTRAINT.receiver().unwrap();
    let mut emergency_rcv = EMERGENCY_SHUTDOWN.receiver().unwrap();
    loop {
        let normal = select4(
            mode_rcv.changed(),
            power_telemetry_rcv.changed(),
            pd_contract_rcv.changed(),
            tx_thermal_rcv.changed(),
        );

        match select(normal, emergency_rcv.changed()).await {
            Either::First(normal_result) => match normal_result {
                Either4::First(mode) => {
                    if let Err(e) = mutex.lock().await.set_mode(mode).await {
                        error(e).await;
                    }
                }
                Either4::Second(telemetry) => {
                    if let Err(e) = mutex.lock().await.set_power_telemetry(telemetry).await {
                        error(e).await;
                    }
                }
                Either4::Third(contract) => {
                    if let Err(e) = mutex.lock().await.set_pd_contract(contract).await {
                        error(e).await;
                    }
                }
                Either4::Fourth(thermal) => {
                    if let Err(e) = mutex.lock().await.set_thermal_constraint(thermal).await {
                        error(e).await;
                    }
                }
            },
            Either::Second(reason) => {
                mutex.lock().await.emergency_off().await;
                match reason {
                    EmergencyReason::VbusOvercurrent => {
                        error("HfAmp: emergency off - VBUS overcurrent").await;
                    }
                    EmergencyReason::PaOvercurrent => {
                        error("HfAmp: emergency off - PA overcurrent").await;
                    }
                    EmergencyReason::Thermal => {
                        error("HfAmp: emergency off - thermal").await;
                    }
                }
            }
        }
    }
}

#[embassy_executor::task]
async fn hf_amp_thermal_task(mutex: &'static Mutex<ThreadModeRawMutex, HfAmp>) {
    loop {
        Timer::after(THERMAL_SAMPLE_PERIOD).await;
        match mutex.lock().await.read_temperatures().await {
            Ok(temps) => {
                if HfAmp::is_thermal_emergency(&temps) {
                    EMERGENCY_SHUTDOWN.sender().send(EmergencyReason::Thermal);
                }
                TX_THERMAL_CONSTRAINT.sender().send(HfAmp::compute_thermal_constraint(&temps));
                PA_TEMPERATURES.sender().send(temps);
            }
            Err(e) => error(e).await,
        }
    }
}
