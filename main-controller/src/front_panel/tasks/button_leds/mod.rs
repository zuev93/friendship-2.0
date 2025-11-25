mod agc_mode_led;
mod mode_led;
mod rf_gain_mode_led;
mod rit_mode_led;
mod tone_led;
mod transmit_led;
mod transmit_mode_led;

use crate::front_panel::config::default_button_mapping;
use crate::front_panel::types::ButtonFunction;

pub fn find_button_id(function: ButtonFunction) -> Option<u8> {
    let mapping = default_button_mapping();
    for (id, func) in mapping.map.iter() {
        if *func == function {
            return Some(*id);
        }
    }
    None
}

pub use agc_mode_led::agc_mode_led_task;
pub use mode_led::mode_led_task;
pub use rf_gain_mode_led::rf_gain_mode_led_task;
pub use rit_mode_led::rit_mode_led_task;
pub use tone_led::tone_led_task;
pub use transmit_led::transmit_led_task;
pub use transmit_mode_led::transmit_mode_led_task;
