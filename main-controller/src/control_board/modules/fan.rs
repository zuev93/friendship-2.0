use embassy_stm32::gpio::OutputType;
use embassy_stm32::peripherals::{PA0, TIM2};
use embassy_stm32::time::Hertz;
use embassy_stm32::timer::low_level::CountingMode;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm};
use embassy_stm32::timer::Channel;
use embassy_stm32::Peri;

const FAN_PWM_FREQ: Hertz = Hertz(25_000);

const TEMP_FAN_OFF_C: i16 = 35;
const TEMP_FAN_MIN_C: i16 = 40;
const TEMP_FAN_MAX_C: i16 = 65;

const DUTY_MIN: u8 = 30;
const DUTY_MAX: u8 = 100;

pub struct Fan {
    pwm: SimplePwm<'static, TIM2>,
    running: bool,
}

impl Fan {
    pub fn new(tim: Peri<'static, TIM2>, pin: Peri<'static, PA0>) -> Self {
        let ch1_pin = PwmPin::new(pin, OutputType::PushPull);
        let pwm = SimplePwm::new(
            tim,
            Some(ch1_pin),
            None,
            None,
            None,
            FAN_PWM_FREQ,
            CountingMode::EdgeAlignedUp,
        );
        Self {
            pwm,
            running: false,
        }
    }

    pub fn update(&mut self, driver_c: i16, final_c: i16) {
        let worst = driver_c.max(final_c);
        let percent = self.compute_duty(worst);

        if percent == 0 {
            if self.running {
                self.pwm.channel(Channel::Ch1).disable();
                self.running = false;
            }
        } else {
            let mut ch = self.pwm.channel(Channel::Ch1);
            ch.set_duty_cycle_percent(percent);
            if !self.running {
                ch.enable();
                self.running = true;
            }
        }
    }

    fn compute_duty(&self, temp_c: i16) -> u8 {
        if temp_c < TEMP_FAN_OFF_C {
            return 0;
        }
        if temp_c < TEMP_FAN_MIN_C {
            if self.running {
                return DUTY_MIN;
            }
            return 0;
        }
        if temp_c >= TEMP_FAN_MAX_C {
            return DUTY_MAX;
        }
        let range = (TEMP_FAN_MAX_C - TEMP_FAN_MIN_C) as u32;
        let above = (temp_c - TEMP_FAN_MIN_C) as u32;
        let duty_range = (DUTY_MAX - DUTY_MIN) as u32;
        DUTY_MIN + (above * duty_range / range) as u8
    }
}
