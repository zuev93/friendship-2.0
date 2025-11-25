use embassy_executor::Spawner;

use crate::{
    hardware::{Led, Leds},
    state::input::{LedSignal, LedState},
};

pub fn spawn_tasks(spawner: &Spawner, leds: Leds, leds_signal: &'static LedSignal) {
    spawner.must_spawn(leds_task(leds, leds_signal));
}

fn set_led(state: LedState, led: &mut Led) {
    match state {
        LedState::Off => {
            led.pin_x.set_low();
            led.pin_y.set_low();
        }
        LedState::Green => {
            led.pin_x.set_low();
            led.pin_y.set_high();
        }
        LedState::Red => {
            led.pin_x.set_high();
            led.pin_y.set_low();
        }
    }
}

#[embassy_executor::task]
async fn leds_task(mut leds: Leds, signal: &'static LedSignal) {
    loop {
        let update = signal.wait().await;
        if (update.led_id as usize) < leds.leds.len() {
            set_led(update.state, &mut leds.leds[update.led_id as usize]);
        }
    }
}
