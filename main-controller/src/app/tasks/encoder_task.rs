use crate::app::events::{CURRENT_BAND, CURRENT_FREQUENCY};
use crate::app::types::{Band, Frequency};
use crate::front_panel::events::{BAND_ENCODER_EVENTS, VFO_ENCODER_EVENTS};
use embassy_futures::select::{select, Either};

#[embassy_executor::task]
pub async fn encoder_task() {
    let band_encoder_rx = BAND_ENCODER_EVENTS.receiver();
    let vfo_encoder_rx = VFO_ENCODER_EVENTS.receiver();

    let mut current_band = Band::Band20m;
    // TODO store in settings
    // TODO add step to config
    let mut current_frequency: Frequency = 14_200_000; // 14.2 MHz
    let step = 1000; // 1 kHz

    let mut last_signaled_band = current_band;
    let mut last_signaled_frequency = current_frequency;

    loop {
        match select(band_encoder_rx.receive(), vfo_encoder_rx.receive()).await {
            Either::First(band_event) => {
                if band_event.delta > 0 {
                    current_band = current_band.next();
                    current_frequency = current_band.lower_frequency();
                } else if band_event.delta < 0 {
                    current_band = current_band.prev();
                    current_frequency = current_band.upper_frequency();
                }

                if current_band != last_signaled_band {
                    CURRENT_BAND.signal(current_band);
                    last_signaled_band = current_band;
                }
                if current_frequency != last_signaled_frequency {
                    CURRENT_FREQUENCY.signal(current_frequency);
                    last_signaled_frequency = current_frequency;
                }
            }

            Either::Second(vfo_event) => {
                let delta_frequency = (vfo_event.delta as i32) * step;
                current_frequency = current_frequency.saturating_add_signed(delta_frequency);

                if current_frequency > current_band.upper_frequency() {
                    current_band = current_band.next();
                    current_frequency = current_band.lower_frequency();
                } else if current_frequency < current_band.lower_frequency() {
                    current_band = current_band.prev();
                    current_frequency = current_band.upper_frequency();
                }

                if current_band != last_signaled_band {
                    CURRENT_BAND.signal(current_band);
                    last_signaled_band = current_band;
                }
                if current_frequency != last_signaled_frequency {
                    CURRENT_FREQUENCY.signal(current_frequency);
                    last_signaled_frequency = current_frequency;
                }
            }
        }
    }
}
