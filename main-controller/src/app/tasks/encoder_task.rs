use crate::app::tasks::arbiters::{
    clarifier_value::CLARIFIER_VALUE_CMD,
    frequency::{BandCommand, BAND_CMD, FREQUENCY_CMD},
    if_gain::IF_GAIN_CMD,
    microphone::MICROPHONE_CMD,
    nb_level::NB_LEVEL_CMD,
    rf_power::RF_POWER_CMD,
    squelch::SQUELCH_CMD,
    volume::VOLUME_CMD,
};
use crate::front_panel::events::ENCODER_EVENTS;
use crate::front_panel::types::EncoderFunction;

const STEP_SIZE: i16 = 4;

#[embassy_executor::task]
pub async fn encoder_task() {
    let encoder_rx = ENCODER_EVENTS.receiver();

    loop {
        let event = encoder_rx.receive().await;

        match event.function {
            EncoderFunction::Band => {
                if event.delta > 0 {
                    BAND_CMD.signal(BandCommand::Up);
                } else if event.delta < 0 {
                    BAND_CMD.signal(BandCommand::Down);
                }
            }
            EncoderFunction::Vfo => {
                let delta = event.delta as i32 * 1000;
                FREQUENCY_CMD.signal(delta);
            }
            EncoderFunction::Volume => {
                VOLUME_CMD.signal(event.delta as i16 * STEP_SIZE);
            }
            EncoderFunction::Microphone => {
                MICROPHONE_CMD.signal(event.delta as i16 * STEP_SIZE);
            }
            EncoderFunction::RfPower => {
                RF_POWER_CMD.signal(event.delta as i16 * STEP_SIZE);
            }
            EncoderFunction::IfGain => {
                IF_GAIN_CMD.signal(event.delta as i16 * STEP_SIZE);
            }
            EncoderFunction::Clarifier => {
                CLARIFIER_VALUE_CMD.signal(event.delta as i16 * STEP_SIZE);
            }
            EncoderFunction::Squelch => {
                SQUELCH_CMD.signal(event.delta as i16 * STEP_SIZE);
            }
            EncoderFunction::NbLevel => {
                NB_LEVEL_CMD.signal(event.delta as i16 * STEP_SIZE);
            }
            _ => {}
        }
    }
}
