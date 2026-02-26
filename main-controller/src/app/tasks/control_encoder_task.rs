use crate::app::events::{
    CURRENT_CLARIFIER_VALUE, CURRENT_IF_GAIN, CURRENT_MICROPHONE, CURRENT_RF_POWER,
    CURRENT_SQUELCH, CURRENT_VOLUME,
};
use crate::app::types::RfPowerPercent;
use crate::front_panel::events::CONTROL_ENCODER_EVENTS;
use crate::front_panel::types::EncoderFunction;

const ACCUMULATOR_MAX: i16 = 1000;
const ACCUMULATOR_MIN: i16 = 0;
const STEP_SIZE: i16 = 4;

fn clamp_accumulator(value: i16) -> i16 {
    if value < ACCUMULATOR_MIN {
        ACCUMULATOR_MIN
    } else if value > ACCUMULATOR_MAX {
        ACCUMULATOR_MAX
    } else {
        value
    }
}

#[embassy_executor::task]
pub async fn control_encoder_task() {
    let mut volume: i16 = 0;
    let mut microphone: i16 = 0;
    let mut rf_power: i16 = 0;
    let mut if_gain: i16 = 0;
    let mut clarifier: i16 = 0;
    let mut squelch: i16 = 0;

    loop {
        let event = CONTROL_ENCODER_EVENTS.receive().await;
        let delta = event.delta as i16 * STEP_SIZE;

        match event.function {
            EncoderFunction::Volume => {
                let new_val = clamp_accumulator(volume + delta);
                if new_val != volume {
                    volume = new_val;
                    CURRENT_VOLUME.signal(volume);
                }
            }
            EncoderFunction::Microphone => {
                let new_val = clamp_accumulator(microphone + delta);
                if new_val != microphone {
                    microphone = new_val;
                    CURRENT_MICROPHONE.signal(microphone);
                }
            }
            EncoderFunction::RfPower => {
                let new_val = clamp_accumulator(rf_power + delta);
                if new_val != rf_power {
                    rf_power = new_val;
                    CURRENT_RF_POWER.signal(RfPowerPercent::from_accumulated(rf_power));
                }
            }
            EncoderFunction::IfGain => {
                let new_val = clamp_accumulator(if_gain + delta);
                if new_val != if_gain {
                    if_gain = new_val;
                    CURRENT_IF_GAIN.signal(if_gain);
                }
            }
            EncoderFunction::Clarifier => {
                let new_val = clamp_accumulator(clarifier + delta);
                if new_val != clarifier {
                    clarifier = new_val;
                    CURRENT_CLARIFIER_VALUE.signal(clarifier);
                }
            }
            EncoderFunction::Squelch => {
                let new_val = clamp_accumulator(squelch + delta);
                if new_val != squelch {
                    squelch = new_val;
                    CURRENT_SQUELCH.signal(squelch);
                }
            }
            _ => {}
        }
    }
}
