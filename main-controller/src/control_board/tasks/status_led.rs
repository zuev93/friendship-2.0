use embassy_executor::Spawner;
use embassy_futures::select::{select4, Either4};
use embassy_stm32::gpio::{Level, Output, Pin, Speed};
use embassy_stm32::Peri;
use embassy_time::{Duration, Timer};

use crate::app::events::MODE;
use crate::runtime_stats::TaskId;
use common::error::{BSOD, ERROR_MESSAGES};
use common::protocol_types::Mode;
use druzhba_macros::instrumented;

const SLOW_BLINK: Duration = Duration::from_millis(500);
const FAST_BLINK: Duration = Duration::from_millis(100);

enum LedState {
    Booting,
    Normal,
    ErrorBurst(u8),
    Fatal,
}

pub fn create_task(spawner: Spawner, pin: Peri<'static, impl Pin>) {
    let led = Output::new(pin, Level::High, Speed::Low);
    spawner.must_spawn(status_led_task(led));
}

#[instrumented(TaskId::StatusLed)]
#[embassy_executor::task]
async fn status_led_task(mut led: Output<'static>) {
    let mut state = LedState::Booting;
    let mut led_on = true;
    let mut mode_rcv = MODE.receiver().unwrap();

    loop {
        match state {
            LedState::Booting => {
                match select4(
                    Timer::after(SLOW_BLINK),
                    BSOD.wait(),
                    ERROR_MESSAGES.receive(),
                    mode_rcv.changed(),
                )
                .await
                {
                    Either4::First(_) => {
                        led_on = !led_on;
                        if led_on {
                            led.set_high();
                        } else {
                            led.set_low();
                        }
                    }
                    Either4::Second(_) => {
                        state = LedState::Fatal;
                    }
                    Either4::Third(_) => {
                        state = LedState::ErrorBurst(6);
                    }
                    Either4::Fourth(mode) => match mode {
                        Mode::Rx | Mode::Tx => {
                            led.set_low();
                            led_on = false;
                            state = LedState::Normal;
                        }
                        Mode::StandBy | Mode::WarmUp => {}
                    },
                }
            }
            LedState::Normal => {
                match embassy_futures::select::select3(
                    BSOD.wait(),
                    ERROR_MESSAGES.receive(),
                    mode_rcv.changed(),
                )
                .await
                {
                    embassy_futures::select::Either3::First(_) => {
                        state = LedState::Fatal;
                    }
                    embassy_futures::select::Either3::Second(_) => {
                        state = LedState::ErrorBurst(6);
                    }
                    embassy_futures::select::Either3::Third(mode) => match mode {
                        Mode::StandBy | Mode::WarmUp => {
                            led.set_high();
                            led_on = true;
                            state = LedState::Booting;
                        }
                        Mode::Rx | Mode::Tx => {}
                    },
                }
            }
            LedState::ErrorBurst(remaining) => {
                if remaining == 0 {
                    if led_on {
                        led.set_low();
                        led_on = false;
                    }
                    state = LedState::Normal;
                    continue;
                }
                led_on = !led_on;
                if led_on {
                    led.set_high();
                } else {
                    led.set_low();
                }
                match embassy_futures::select::select(
                    Timer::after(FAST_BLINK),
                    BSOD.wait(),
                )
                .await
                {
                    embassy_futures::select::Either::First(_) => {
                        state = LedState::ErrorBurst(remaining - 1);
                    }
                    embassy_futures::select::Either::Second(_) => {
                        state = LedState::Fatal;
                    }
                }
            }
            LedState::Fatal => {
                led_on = !led_on;
                if led_on {
                    led.set_high();
                } else {
                    led.set_low();
                }
                Timer::after(FAST_BLINK).await;
            }
        }
    }
}
