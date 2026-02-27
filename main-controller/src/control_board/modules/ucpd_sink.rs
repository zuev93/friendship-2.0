use embassy_stm32::peripherals::UCPD1;
use embassy_stm32::ucpd::{self, PdPhy};
use usbpd_traits::{Driver, DriverRxError, DriverTxError};

pub struct UcpdSinkDriver<'d> {
    pd_phy: PdPhy<'d, UCPD1>,
}

impl<'d> UcpdSinkDriver<'d> {
    pub fn new(pd_phy: PdPhy<'d, UCPD1>) -> Self {
        Self { pd_phy }
    }
}

impl Driver for UcpdSinkDriver<'_> {
    async fn wait_for_vbus(&mut self) {}

    async fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, DriverRxError> {
        self.pd_phy.receive(buffer).await.map_err(|err| match err {
            ucpd::RxError::Crc | ucpd::RxError::Overrun => DriverRxError::Discarded,
            ucpd::RxError::HardReset => DriverRxError::HardReset,
        })
    }

    async fn transmit(&mut self, data: &[u8]) -> Result<(), DriverTxError> {
        self.pd_phy.transmit(data).await.map_err(|err| match err {
            ucpd::TxError::Discarded => DriverTxError::Discarded,
            ucpd::TxError::HardReset => DriverTxError::HardReset,
        })
    }

    async fn transmit_hard_reset(&mut self) -> Result<(), DriverTxError> {
        self.pd_phy
            .transmit_hardreset()
            .await
            .map_err(|err| match err {
                ucpd::TxError::Discarded => DriverTxError::Discarded,
                ucpd::TxError::HardReset => DriverTxError::HardReset,
            })
    }
}
