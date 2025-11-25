use crate::app::events::{
    CURRENT_CLARIFIER_VALUE, CURRENT_IF_GAIN, CURRENT_MICROPHONE, CURRENT_RF_POWER,
    CURRENT_SQUELCH, CURRENT_VOLUME,
};
use crate::app::types::{ClarifierValue, IfGain, Microphone, RfPowerPercent, Squelch, Volume};
use crate::front_panel::events::POTENTIOMETER_EVENTS;
use crate::front_panel::types::PotentiometerFunction;

#[embassy_executor::task]
pub async fn potentiometer_task() {
    let mut last_volume: Volume = 0;
    let mut last_microphone: Microphone = 0;
    let mut last_rf_power: RfPowerPercent = RfPowerPercent::new(0);
    let mut last_if_gain: IfGain = 0;
    let mut last_clarifier_value: ClarifierValue = 0;
    let mut last_squelch: Squelch = 0;

    loop {
        let event = POTENTIOMETER_EVENTS.receive().await;

        match event.function {
            PotentiometerFunction::Volume => {
                let volume = event.value;
                if volume != last_volume {
                    CURRENT_VOLUME.signal(volume);
                    last_volume = volume;
                }
            }

            PotentiometerFunction::Microphone => {
                let microphone = event.value;
                if microphone != last_microphone {
                    CURRENT_MICROPHONE.signal(microphone);
                    last_microphone = microphone;
                }
            }

            PotentiometerFunction::RfPower => {
                let rf_power = RfPowerPercent::from_adc_raw(event.value);
                if rf_power != last_rf_power {
                    CURRENT_RF_POWER.signal(rf_power);
                    last_rf_power = rf_power;
                }
            }

            PotentiometerFunction::IfGain => {
                let if_gain = event.value;
                if if_gain != last_if_gain {
                    CURRENT_IF_GAIN.signal(if_gain);
                    last_if_gain = if_gain;
                }
            }

            PotentiometerFunction::Clarifier => {
                let clarifier_value = event.value;
                if clarifier_value != last_clarifier_value {
                    CURRENT_CLARIFIER_VALUE.signal(clarifier_value);
                    last_clarifier_value = clarifier_value;
                }
            }

            PotentiometerFunction::Squelch => {
                let squelch = event.value;
                if squelch != last_squelch {
                    CURRENT_SQUELCH.signal(squelch);
                    last_squelch = squelch;
                }
            }
        }
    }
}
