use embassy_futures::select::{select3, Either3};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::events::{FILTER, SWEEP_ACTIVE, TRANSMIT_MODE};
use crate::app::types::FilterType;

pub enum FilterCommand {
    Cycle,
}

pub static FILTER_CMD: Signal<ThreadModeRawMutex, FilterCommand> = Signal::new();

#[embassy_executor::task]
pub async fn filter_arbiter_task() {
    let mut filter = FilterType::Single;
    let mut user_filter = filter;

    loop {
        match select3(FILTER_CMD.wait(), TRANSMIT_MODE.wait(), SWEEP_ACTIVE.wait()).await {
            Either3::First(cmd) => match cmd {
                FilterCommand::Cycle => {
                    filter = filter.next();
                    user_filter = filter;
                    FILTER.signal(filter);
                }
            },
            Either3::Second(transmit_mode) => {
                filter = FilterType::default_for_mode(transmit_mode);
                user_filter = filter;
                FILTER.signal(filter);
            }
            Either3::Third(active) => {
                if active {
                    user_filter = filter;
                    filter = FilterType::Single;
                    FILTER.signal(filter);
                } else {
                    filter = user_filter;
                    FILTER.signal(filter);
                }
            }
        }
    }
}
