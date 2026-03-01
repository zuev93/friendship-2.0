use crate::app::events::{FREQUENCY, MODE, TRANSMIT_MODE};
use crate::app::tasks::arbiters::frequency::{FrequencyCommand, FREQUENCY_CMD};
use crate::app::tasks::arbiters::mode::{ModeCommand, MODE_CMD};
use crate::app::tasks::arbiters::transmit_mode::{TransmitModeCommand, TRANSMIT_MODE_CMD};
use crate::app::types::Mode;

use super::parser::CatCommand;
use super::response::CatResponse;

pub struct CatHandler;

impl CatHandler {
    pub fn new() -> Self {
        Self
    }

    pub fn handle(&mut self, cmd: CatCommand) -> CatResponse {
        match cmd {
            CatCommand::Id => CatResponse::Id,
            CatCommand::FrequencyRead => {
                let freq = FREQUENCY.try_get().unwrap_or(14_200_000);
                CatResponse::Frequency(freq)
            }
            CatCommand::FrequencyWrite(freq) => {
                FREQUENCY_CMD.signal(FrequencyCommand::SetAbsolute(freq));
                CatResponse::Frequency(freq)
            }
            CatCommand::ModeRead => {
                let mode = TRANSMIT_MODE.try_get().unwrap_or(crate::app::types::TransmitMode::Usb);
                CatResponse::ModeValue(mode)
            }
            CatCommand::ModeWrite(mode) => {
                TRANSMIT_MODE_CMD.signal(TransmitModeCommand::Set(mode));
                CatResponse::ModeValue(mode)
            }
            CatCommand::InfoRead => {
                let frequency = FREQUENCY.try_get().unwrap_or(14_200_000);
                let mode = TRANSMIT_MODE.try_get().unwrap_or(crate::app::types::TransmitMode::Usb);
                let transmitting = MODE.try_get().map(|m| m == Mode::Tx).unwrap_or(false);
                CatResponse::Info {
                    frequency,
                    mode,
                    transmitting,
                }
            }
            CatCommand::Transmit => {
                MODE_CMD.signal(ModeCommand::TransmitPress);
                CatResponse::Tx
            }
            CatCommand::Receive => {
                MODE_CMD.signal(ModeCommand::TransmitRelease);
                CatResponse::Rx
            }
        }
    }
}
