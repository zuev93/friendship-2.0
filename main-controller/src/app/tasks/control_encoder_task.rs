use crate::app::events::{
    CURRENT_CLARIFIER_VALUE, CURRENT_IF_GAIN, CURRENT_MICROPHONE, CURRENT_NB_LEVEL,
    CURRENT_RF_POWER, CURRENT_SQUELCH, CURRENT_VOLUME,
};
use crate::app::types::{
    ClarifierValue, IfGain, Microphone, NbLevel, RfPowerPercent, Squelch, Volume,
};
use crate::front_panel::events::CONTROL_ENCODER_EVENTS;
use crate::front_panel::types::EncoderFunction;

const STEP_SIZE: i16 = 4;

#[embassy_executor::task]
pub async fn control_encoder_task() {
    let mut volume = Volume::new(0);
    let mut microphone = Microphone::new(0);
    let mut rf_power_raw: i16 = 0;
    let mut if_gain = IfGain::new(0);
    let mut clarifier = ClarifierValue::new(0);
    let mut squelch = Squelch::new(0);
    let mut nb_level = NbLevel::new(0);

    loop {
        let event = CONTROL_ENCODER_EVENTS.receive().await;
        let delta = event.delta as i16 * STEP_SIZE;

        match event.function {
            EncoderFunction::Volume => {
                let new_val = Volume::new(volume.raw() + delta);
                if new_val != volume {
                    volume = new_val;
                    CURRENT_VOLUME.signal(volume);
                }
            }
            EncoderFunction::Microphone => {
                let new_val = Microphone::new(microphone.raw() + delta);
                if new_val != microphone {
                    microphone = new_val;
                    CURRENT_MICROPHONE.signal(microphone);
                }
            }
            EncoderFunction::RfPower => {
                let new_raw = (rf_power_raw + delta).clamp(0, 1000);
                if new_raw != rf_power_raw {
                    rf_power_raw = new_raw;
                    CURRENT_RF_POWER.signal(RfPowerPercent::from_accumulated(rf_power_raw));
                }
            }
            EncoderFunction::IfGain => {
                let new_val = IfGain::new(if_gain.raw() + delta);
                if new_val != if_gain {
                    if_gain = new_val;
                    CURRENT_IF_GAIN.signal(if_gain);
                }
            }
            EncoderFunction::Clarifier => {
                let new_val = ClarifierValue::new(clarifier.raw() + delta);
                if new_val != clarifier {
                    clarifier = new_val;
                    CURRENT_CLARIFIER_VALUE.signal(clarifier);
                }
            }
            EncoderFunction::Squelch => {
                let new_val = Squelch::new(squelch.raw() + delta);
                if new_val != squelch {
                    squelch = new_val;
                    CURRENT_SQUELCH.signal(squelch);
                }
            }
            EncoderFunction::NbLevel => {
                let new_val = NbLevel::new(nb_level.raw() + delta);
                if new_val != nb_level {
                    nb_level = new_val;
                    CURRENT_NB_LEVEL.signal(nb_level);
                }
            }
            _ => {}
        }
    }
}
