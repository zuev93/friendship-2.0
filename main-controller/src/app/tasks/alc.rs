use embassy_futures::select::{select4, Either4};
use embassy_time::Timer;

use crate::app::events::{ALC_CONSTRAINT, COUPLER_METRICS, MODE, RF_POWER};
use crate::app::types::{CouplerMetrics, Mode};

const MAX_OUTPUT_W: f32 = 50.0;
const SWR_FOLDBACK_START: f32 = 2.0;
const SWR_FOLDBACK_FULL: f32 = 3.0;
const SWR_SHUTDOWN: f32 = 4.0;
const REDUCE_RATE: i32 = 500;
const RECOVER_RATE: i32 = 100;
const STALE_TICKS: u8 = 5;

struct AlcState {
    alc_cp: i32,
    user_power_cp: u16,
    mode: Mode,
    stale_ticks: u8,
}

impl AlcState {
    fn new() -> Self {
        Self {
            alc_cp: 10000,
            user_power_cp: 0,
            mode: Mode::StandBy,
            stale_ticks: 0,
        }
    }

    fn reset(&mut self) {
        self.alc_cp = 10000;
        self.stale_ticks = 0;
    }
}

fn swr_foldback(vswr: f32) -> i32 {
    if vswr >= SWR_SHUTDOWN {
        return 0;
    }
    if vswr <= SWR_FOLDBACK_START {
        return 10000;
    }
    if vswr >= SWR_FOLDBACK_FULL {
        return 1000;
    }
    let range = SWR_FOLDBACK_FULL - SWR_FOLDBACK_START;
    let fraction = (vswr - SWR_FOLDBACK_START) / range;
    let scale = 10000.0 - fraction * 9000.0;
    scale as i32
}

fn compute_target_w(user_power_cp: u16) -> f32 {
    (user_power_cp as f32 / 10000.0) * MAX_OUTPUT_W
}

fn compute_margin(target_w: f32) -> f32 {
    if target_w * 0.05 > 0.5 {
        target_w * 0.05
    } else {
        0.5
    }
}

fn run_alc(state: &mut AlcState, metrics: &CouplerMetrics) {
    let target_w = compute_target_w(state.user_power_cp);
    let swr_scale = swr_foldback(metrics.vswr);
    let effective_target = target_w * swr_scale as f32 / 10000.0;
    let margin = compute_margin(effective_target);

    if metrics.forward_w > effective_target + margin {
        state.alc_cp = (state.alc_cp - REDUCE_RATE).max(0);
    } else if metrics.forward_w < effective_target - margin {
        state.alc_cp = (state.alc_cp + RECOVER_RATE).min(10000);
    }
}

#[embassy_executor::task]
pub async fn alc_task() {
    let mut state = AlcState::new();

    loop {
        match select4(
            COUPLER_METRICS.wait(),
            MODE.wait(),
            RF_POWER.wait(),
            Timer::after_millis(200),
        )
        .await
        {
            Either4::First(metrics) => {
                state.stale_ticks = 0;
                if state.mode == Mode::Tx {
                    run_alc(&mut state, &metrics);
                    ALC_CONSTRAINT.signal(state.alc_cp);
                }
            }
            Either4::Second(mode) => {
                state.mode = mode;
                if mode != Mode::Tx {
                    state.reset();
                    ALC_CONSTRAINT.signal(state.alc_cp);
                }
            }
            Either4::Third(rf_power) => {
                state.user_power_cp = rf_power.centipercent;
            }
            Either4::Fourth(()) => {
                if state.mode == Mode::Tx {
                    state.stale_ticks = state.stale_ticks.saturating_add(1);
                    if state.stale_ticks >= STALE_TICKS {
                        state.alc_cp = 0;
                        ALC_CONSTRAINT.signal(state.alc_cp);
                    }
                }
            }
        }
    }
}
