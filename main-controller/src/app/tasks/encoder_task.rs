use crate::app::tasks::arbiters::{
    clarifier_value::CLARIFIER_VALUE_CMD,
    compression::COMPRESSION_CMD,
    cw_peak_width::CW_PEAK_WIDTH_CMD,
    cw_pitch::CW_PITCH_CMD,
    dsp_bandwidth::DSP_BANDWIDTH_CMD,
    dsp_shift::DSP_SHIFT_CMD,
    frequency::{BandCommand, FrequencyCommand, BAND_CMD, FREQUENCY_CMD},
    if_gain::IF_GAIN_CMD,
    microphone::MICROPHONE_CMD,
    nb_level::NB_LEVEL_CMD,
    nr_level::NR_LEVEL_CMD,
    rf_power::RF_POWER_CMD,
    rx_eq::{RxEqCommand, RX_EQ_CMD},
    scan::{ScanCommand, SCAN_CMD},
    squelch::{SquelchCommand, SQUELCH_CMD},
    tx_eq::{TxEqCommand, TX_EQ_CMD},
    volume::VOLUME_CMD,
    vox::{VoxCommand, VOX_CMD},
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
                SCAN_CMD.signal(ScanCommand::Stop);
                if event.delta > 0 {
                    BAND_CMD.signal(BandCommand::Up);
                } else if event.delta < 0 {
                    BAND_CMD.signal(BandCommand::Down);
                }
            }
            EncoderFunction::Vfo => {
                SCAN_CMD.signal(ScanCommand::Stop);
                let delta = event.delta as i32 * 1000;
                FREQUENCY_CMD.signal(FrequencyCommand::Delta(delta));
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
                SQUELCH_CMD.signal(SquelchCommand::Delta(event.delta as i16 * STEP_SIZE));
            }
            EncoderFunction::NbLevel => {
                NB_LEVEL_CMD.signal(event.delta as i16 * STEP_SIZE);
            }
            EncoderFunction::NrLevel => {
                NR_LEVEL_CMD.signal(event.delta as i16 * STEP_SIZE);
            }
            EncoderFunction::Compression => {
                COMPRESSION_CMD.signal(event.delta as i16 * STEP_SIZE);
            }
            EncoderFunction::DspBandwidth => {
                DSP_BANDWIDTH_CMD.signal(event.delta as i16 * STEP_SIZE);
            }
            EncoderFunction::DspShift => {
                DSP_SHIFT_CMD.signal(event.delta as i16 * STEP_SIZE);
            }
            EncoderFunction::CwPeakWidth => {
                CW_PEAK_WIDTH_CMD.signal(event.delta as i16 * STEP_SIZE);
            }
            EncoderFunction::CwPitch => {
                CW_PITCH_CMD.signal(event.delta as i16 * STEP_SIZE);
            }
            EncoderFunction::TxEqLow => {
                TX_EQ_CMD.signal(TxEqCommand::SetLow(event.delta as i8));
            }
            EncoderFunction::TxEqMid => {
                TX_EQ_CMD.signal(TxEqCommand::SetMid(event.delta as i8));
            }
            EncoderFunction::TxEqHigh => {
                TX_EQ_CMD.signal(TxEqCommand::SetHigh(event.delta as i8));
            }
            EncoderFunction::RxEqLow => {
                RX_EQ_CMD.signal(RxEqCommand::SetLow(event.delta as i8));
            }
            EncoderFunction::RxEqMid => {
                RX_EQ_CMD.signal(RxEqCommand::SetMid(event.delta as i8));
            }
            EncoderFunction::RxEqHigh => {
                RX_EQ_CMD.signal(RxEqCommand::SetHigh(event.delta as i8));
            }
            EncoderFunction::VoxGain => {
                VOX_CMD.signal(VoxCommand::GainDelta(event.delta as i16 * STEP_SIZE));
            }
            EncoderFunction::VoxDelay => {
                VOX_CMD.signal(VoxCommand::DelayDelta(event.delta as i16 * STEP_SIZE));
            }
            EncoderFunction::VoxAntiTrip => {
                VOX_CMD.signal(VoxCommand::AntiTripDelta(event.delta as i16 * STEP_SIZE));
            }
            EncoderFunction::ScanStep => {
                SCAN_CMD.signal(ScanCommand::StepDelta(event.delta as i16 * STEP_SIZE));
            }
            EncoderFunction::ScanResume => {
                SCAN_CMD.signal(ScanCommand::ResumeDelta(event.delta as i16));
            }
            EncoderFunction::Menu => {}
        }
    }
}
