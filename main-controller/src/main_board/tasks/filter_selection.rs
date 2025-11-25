use embassy_futures::select::{select, Either};

use crate::{
    app::events::{CURRENT_FILTER, CURRENT_MODE},
    main_board::modules::filter_select::FilterSelect,
};
use common::error::error;

#[embassy_executor::task]
pub async fn filter_selection_task(mut filter_select: FilterSelect) {
    loop {
        match select(CURRENT_FILTER.wait(), CURRENT_MODE.wait()).await {
            Either::First(filter) => {
                if let Err(e) = filter_select.set_filter(filter).await {
                    error(e).await;
                }
            }
            Either::Second(mode) => {
                if let Err(e) = filter_select.set_mode(mode).await {
                    error(e).await;
                }
            }
        }
    }
}
