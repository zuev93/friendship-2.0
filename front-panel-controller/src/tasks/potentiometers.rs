use embassy_executor::Spawner;
use embassy_time::Timer;

use crate::constants::POTENTIOMETERS_POLL_INTERVAL;
use crate::hardware::Potentiometers;
use crate::state::output::{OutputEvent, PotentiometerValue, OUTPUT_EVENTS};

pub fn spawn_tasks(spawner: &Spawner, pots: Potentiometers) {
    spawner.must_spawn(potentiometers_task(pots));
}

#[embassy_executor::task]
async fn potentiometers_task(mut pots: Potentiometers) {
    let adc1 = &mut pots.adc1;
    let mut prev_values = [0u16; 6];

    loop {
        Timer::after(POTENTIOMETERS_POLL_INTERVAL).await;

        let values = [
            adc1.read(&mut pots.var1),
            adc1.read(&mut pots.var2),
            adc1.read(&mut pots.var3),
            adc1.read(&mut pots.var4),
            adc1.read(&mut pots.var5),
            adc1.read(&mut pots.var6),
        ];

        for (i, &value) in values.iter().enumerate() {
            if value != prev_values[i] {
                OUTPUT_EVENTS
                    .send(OutputEvent::Potentiometer(PotentiometerValue {
                        id: (i + 1) as u8,
                        value,
                    }))
                    .await;
                prev_values[i] = value;
            }
        }
    }
}
