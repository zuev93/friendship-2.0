use embassy_executor::Spawner;
use embassy_futures::select::{select, Either};
use embassy_stm32::peripherals::{GPDMA1_CH6, GPDMA1_CH7, PB13, PB14, UCPD1};
use embassy_stm32::ucpd::{CcPhy, CcPull, CcSel, CcVState, Ucpd};
use embassy_stm32::Peri;
use embassy_time::{Duration, Timer};

use crate::control_board::modules::ucpd_policy::TransceiverPolicyManager;
use crate::control_board::modules::ucpd_sink::UcpdSinkDriver;
use crate::control_board::modules::ucpd_timer::EmbassySinkTimer;
use crate::hardware::Irqs;
use crate::runtime_stats::TaskId;
use common::error::error;
use druzhba_macros::instrumented;
use usbpd::sink::policy_engine::Sink;

const DEBOUNCE_MS: u64 = 100;

pub fn create_tasks(
    spawner: Spawner,
    ucpd: Peri<'static, UCPD1>,
    cc1: Peri<'static, PB13>,
    cc2: Peri<'static, PB14>,
    rx_dma: Peri<'static, GPDMA1_CH6>,
    tx_dma: Peri<'static, GPDMA1_CH7>,
) {
    spawner.must_spawn(ucpd_task(ucpd, cc1, cc2, rx_dma, tx_dma));
}

#[instrumented(TaskId::UcpdTask)]
#[embassy_executor::task]
async fn ucpd_task(
    mut ucpd_peri: Peri<'static, UCPD1>,
    mut cc1: Peri<'static, PB13>,
    mut cc2: Peri<'static, PB14>,
    mut rx_dma: Peri<'static, GPDMA1_CH6>,
    mut tx_dma: Peri<'static, GPDMA1_CH7>,
) {
    loop {
        let mut ucpd = Ucpd::new(
            ucpd_peri.reborrow(),
            Irqs,
            cc1.reborrow(),
            cc2.reborrow(),
            Default::default(),
        );

        ucpd.cc_phy().set_pull(CcPull::Sink);

        let cc_sel = wait_attached(ucpd.cc_phy()).await;

        let (mut cc_phy, pd_phy) =
            ucpd.split_pd_phy(rx_dma.reborrow(), tx_dma.reborrow(), cc_sel);

        let driver = UcpdSinkDriver::new(pd_phy);
        let policy = TransceiverPolicyManager::new();
        let mut sink: Sink<UcpdSinkDriver<'_>, EmbassySinkTimer, _> = Sink::new(driver, policy);

        match select(sink.run(), wait_detached(&mut cc_phy, cc_sel)).await {
            Either::First(result) => {
                if let Err(_e) = result {
                    error("UCPD: sink run error").await;
                }
            }
            Either::Second(()) => {}
        }

        drop(cc_phy);
        Timer::after(Duration::from_millis(DEBOUNCE_MS)).await;
    }
}

async fn wait_attached(cc_phy: &mut CcPhy<'_, UCPD1>) -> CcSel {
    loop {
        let (cc1, cc2) = cc_phy.vstate();
        if is_attached(cc1) {
            Timer::after(Duration::from_millis(DEBOUNCE_MS)).await;
            let (cc1_confirm, _) = cc_phy.vstate();
            if is_attached(cc1_confirm) {
                return CcSel::CC1;
            }
        }
        if is_attached(cc2) {
            Timer::after(Duration::from_millis(DEBOUNCE_MS)).await;
            let (_, cc2_confirm) = cc_phy.vstate();
            if is_attached(cc2_confirm) {
                return CcSel::CC2;
            }
        }
        cc_phy.wait_for_vstate_change().await;
    }
}

async fn wait_detached(cc_phy: &mut CcPhy<'_, UCPD1>, cc_sel: CcSel) {
    loop {
        let (cc1, cc2) = cc_phy.wait_for_vstate_change().await;
        let active = match cc_sel {
            CcSel::CC1 => cc1,
            CcSel::CC2 => cc2,
        };
        if !is_attached(active) {
            Timer::after(Duration::from_millis(DEBOUNCE_MS)).await;
            let (cc1_check, cc2_check) = cc_phy.vstate();
            let confirm = match cc_sel {
                CcSel::CC1 => cc1_check,
                CcSel::CC2 => cc2_check,
            };
            if !is_attached(confirm) {
                return;
            }
        }
    }
}

fn is_attached(vstate: CcVState) -> bool {
    matches!(vstate, CcVState::LOW | CcVState::HIGH | CcVState::HIGHEST)
}
