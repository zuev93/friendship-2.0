use embassy_executor::Spawner;
use embassy_stm32::dac::Value;

use crate::{hardware::SMeter, state::input::SMeterSignal};

pub fn spawn_tasks(spawner: &Spawner, s_meter: SMeter, s_meter_signal: &'static SMeterSignal) {
    spawner.must_spawn(s_meter_task(s_meter, s_meter_signal));
}

#[embassy_executor::task]
async fn s_meter_task(mut s_meter: SMeter, s_meter_signal: &'static SMeterSignal) {
    loop {
        let value = s_meter_signal.wait().await;
        s_meter.dac.set(Value::Bit12Left(value));
    }
}
