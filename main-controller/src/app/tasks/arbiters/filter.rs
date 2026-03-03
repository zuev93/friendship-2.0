use druzhba_macros::instrumented;
use crate::runtime_stats::TaskId;
use embassy_futures::select::{select3, Either3};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::events::{FILTER, SWEEP_ACTIVE, TRANSMIT_MODE};
use crate::app::types::FilterType;

pub enum FilterCommand {
    Cycle,
    Set(FilterType),
}

pub static FILTER_CMD: Signal<ThreadModeRawMutex, FilterCommand> = Signal::new();

#[instrumented(TaskId::FilterArbiter)]
#[embassy_executor::task]
pub async fn filter_arbiter_task() {
    let mut filter = FilterType::Narrow;
    let mut user_filter = filter;
    let mut transmit_mode_rcv = TRANSMIT_MODE.receiver().unwrap();
    let mut sweep_active_rcv = SWEEP_ACTIVE.receiver().unwrap();

    loop {
        match select3(
            FILTER_CMD.wait(),
            transmit_mode_rcv.changed(),
            sweep_active_rcv.changed(),
        )
        .await
        {
            Either3::First(cmd) => match cmd {
                FilterCommand::Cycle => {
                    filter = filter.next();
                    user_filter = filter;
                    FILTER.sender().send(filter);
                }
                FilterCommand::Set(f) => {
                    filter = f;
                    user_filter = filter;
                    FILTER.sender().send(filter);
                }
            },
            Either3::Second(transmit_mode) => {
                filter = FilterType::default_for_mode(transmit_mode);
                user_filter = filter;
                FILTER.sender().send(filter);
            }
            Either3::Third(active) => {
                if active {
                    user_filter = filter;
                    filter = FilterType::Wide;
                    FILTER.sender().send(filter);
                } else {
                    filter = user_filter;
                    FILTER.sender().send(filter);
                }
            }
        }
    }
}
