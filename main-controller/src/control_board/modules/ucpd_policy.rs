use crate::control_board::events::{PdContract, PD_CONTRACT};
use usbpd::_250milliwatts_mod::_250milliwatts;
use usbpd::_50millivolts_mod::_50millivolts;
use usbpd::protocol_layer::message::data::request::{CurrentRequest, PowerSource, VoltageRequest};
use usbpd::protocol_layer::message::data::source_capabilities::SourceCapabilities;
use usbpd::sink::device_policy_manager::{self, Event};
use usbpd::units::{ElectricPotential, Power};

const EPR_OPERATIONAL_PDP_250MW: u32 = 800;
const MAX_VOLTAGE_50MV: u32 = 960;

pub struct TransceiverPolicyManager {
    epr_entry_requested: bool,
}

impl TransceiverPolicyManager {
    pub fn new() -> Self {
        Self {
            epr_entry_requested: false,
        }
    }
}

impl device_policy_manager::DevicePolicyManager for TransceiverPolicyManager {
    async fn request(&mut self, source_capabilities: &SourceCapabilities) -> PowerSource {
        if let Ok(avs) = PowerSource::new_epr_avs(
            CurrentRequest::Highest,
            ElectricPotential::new::<_50millivolts>(MAX_VOLTAGE_50MV),
            source_capabilities,
        ) {
            return avs;
        }

        PowerSource::new_fixed(
            CurrentRequest::Highest,
            VoltageRequest::Highest,
            source_capabilities,
        )
        .unwrap_or_else(|_| {
            PowerSource::new_fixed(
                CurrentRequest::Highest,
                VoltageRequest::Safe5V,
                source_capabilities,
            )
            .unwrap()
        })
    }

    async fn transition_power(&mut self, accepted: &PowerSource) {
        let contract = match accepted {
            PowerSource::FixedVariableSupply(fvs) => {
                let current_ma = fvs.raw_operating_current() as u32 * 10;
                PdContract {
                    voltage_mv: 5000,
                    current_ma,
                    power_mw: (5000 * current_ma) / 1000,
                }
            }
            _ => PdContract::default(),
        };
        PD_CONTRACT.sender().send(contract);
    }

    async fn hard_reset(&mut self) {
        self.epr_entry_requested = false;
        PD_CONTRACT.sender().send(PdContract::default());
    }

    async fn get_event(&mut self, source_capabilities: &SourceCapabilities) -> Event {
        if !self.epr_entry_requested && source_capabilities.epr_mode_capable() {
            self.epr_entry_requested = true;
            return Event::EnterEprMode(Power::new::<_250milliwatts>(EPR_OPERATIONAL_PDP_250MW));
        }
        core::future::pending().await
    }
}
