use embassy_executor::Spawner;
use embassy_futures::select::select;
use embassy_time::Timer;

use crate::constants::QEI_POLL_INTERVAL;
use crate::hardware::{ExtiEncoder, ExtiEncoders, QeiEncoder, QeiEncoders};
use crate::state::output::{EncoderDirection, EncoderEvent, OutputEvent, OUTPUT_EVENTS};

const QEI_BASE_ID: u8 = 0;
const EXTI_BASE_ID: u8 = 4;

pub fn spawn_tasks(
    spawner: &Spawner,
    mut qei_encoders: QeiEncoders,
    mut exti_encoders: ExtiEncoders,
) {
    for i in 0..qei_encoders.encoders.len() {
        if let Some(encoder) = qei_encoders.encoders[i].take() {
            spawner.must_spawn(qei_encoder_task(QEI_BASE_ID + i as u8, encoder));
        }
    }

    for i in 0..exti_encoders.encoders.len() {
        if let Some(encoder) = exti_encoders.encoders[i].take() {
            spawner.must_spawn(exti_encoder_task(EXTI_BASE_ID + i as u8, encoder));
        }
    }
}

#[embassy_executor::task(pool_size = 4)]
async fn qei_encoder_task(encoder_id: u8, encoder: QeiEncoder) {
    let mut last_count: u16 = encoder.count();

    loop {
        Timer::after(QEI_POLL_INTERVAL).await;

        let current = encoder.count();
        let delta = current.wrapping_sub(last_count) as i16;

        if delta == 0 {
            continue;
        }

        last_count = current;

        let (direction, steps) = if delta > 0 && delta < 0x7FFF {
            (EncoderDirection::Clockwise, delta as i8)
        } else {
            (EncoderDirection::CounterClockwise, -(delta.unsigned_abs() as i8))
        };

        OUTPUT_EVENTS
            .send(OutputEvent::Encoder(EncoderEvent {
                id: encoder_id,
                direction,
                steps,
            }))
            .await;
    }
}

#[embassy_executor::task(pool_size = 5)]
async fn exti_encoder_task(encoder_id: u8, mut encoder: ExtiEncoder) {
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
