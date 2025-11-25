use embassy_futures::select::{select3, Either3};

use crate::{
    app::events::{CURRENT_FILTER, CURRENT_MODE, CURRENT_TRANSMIT_MODE},
    main_board::modules::if_reference::IfReference,
};
use common::error::error;

#[embassy_executor::task]
pub async fn if_reference_task(mut if_reference: IfReference) {
    loop {
        match select3(
            CURRENT_TRANSMIT_MODE.wait(),
            CURRENT_MODE.wait(),
            CURRENT_FILTER.wait(),
        )
        .await
        {
            Either3::First(transmit_mode) => {
                if let Err(e) = if_reference.set_transmit_mode(transmit_mode).await {
                    error(e).await;
                }
            }
            Either3::Second(mode) => {
                if let Err(e) = if_reference.set_mode(mode).await {
                    error(e).await;
                }
            }
            Either3::Third(filter) => {
                if let Err(e) = if_reference.set_filter(filter).await {
                    error(e).await;
                }
            }
        }
    }
}
