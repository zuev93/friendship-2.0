use embassy_executor::Spawner;
use embassy_futures::select::select;

use crate::hardware::{Encoder, Encoders};
use crate::state::output::{EncoderDirection, EncoderEvent, OutputEvent, OUTPUT_EVENTS};

pub fn spawn_tasks(spawner: &Spawner, mut encoders: Encoders) {
    for i in 0..encoders.encoders.len() {
        if let Some(encoder) = encoders.encoders[i].take() {
            spawner.must_spawn(encoder_task(i as u8, encoder));
        }
    }
}

#[embassy_executor::task]
async fn encoder_task(encoder_id: u8, mut encoder: Encoder) {
    loop {
        select(
            encoder.channel_a.wait_for_any_edge(),
            encoder.channel_b.wait_for_any_edge(),
        )
        .await;

        let a_state = encoder.channel_a.is_low();
        let b_state = encoder.channel_b.is_low();

        let direction = if a_state != b_state {
            EncoderDirection::Clockwise
        } else {
            EncoderDirection::CounterClockwise
        };

        OUTPUT_EVENTS
            .send(OutputEvent::Encoder(EncoderEvent {
                id: encoder_id,
                direction,
                steps: 1,
            }))
            .await;
    }
}
