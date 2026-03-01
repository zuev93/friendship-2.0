use embassy_time::{Duration, Instant, Timer};

use crate::app::{
    events::{FREQUENCY, SQUELCH, SWEEP_ACTIVE, SWEEP_REQUEST, WATERFALL_LINE},
    types::{SweepRequest, WaterfallBuffer},
    waterfall::WaterfallSweeper,
};
use crate::main_board::events::CURRENT_RSSI;

const DEFAULT_SPAN_HZ: u32 = 100_000;
const FULL_LINE_BUDGET_MS: u64 = 130;
const LISTENING_WINDOW_MS: u64 = 100;
const SQUELCH_TRACK_WINDOW_MS: u64 = 1000;

fn squelch_to_dbm(raw: i16) -> i8 {
    const DBM_MIN: i32 = -120;
    const DBM_MAX: i32 = -20;
    const RAW_MAX: i32 = 1000;
    if raw <= 0 {
        return -128;
    }
    (DBM_MIN + (raw as i32 * (DBM_MAX - DBM_MIN) / RAW_MAX)) as i8
}

#[embassy_executor::task]
pub async fn sweep_scheduler_task() {
    let mut sweeper = WaterfallSweeper::new(DEFAULT_SPAN_HZ);
    let mut buffer = WaterfallBuffer::new();
    let mut vfo_freq: u32 = 7_100_000;
    let mut squelch_threshold_dbm: i8 = -128;
    let mut squelch_closed_since: Option<Instant> = None;
    let mut frequency_rcv = FREQUENCY.anon_receiver();
    let mut squelch_rcv = SQUELCH.anon_receiver();
    let mut rssi_anon_rcv = CURRENT_RSSI.anon_receiver();
    let mut rssi_rcv = CURRENT_RSSI.receiver().unwrap();

    loop {
        if let Some(freq) = frequency_rcv.try_changed() {
            vfo_freq = freq;
        }
        if let Some(squelch) = squelch_rcv.try_changed() {
            squelch_threshold_dbm = squelch_to_dbm(squelch.raw());
        }

        if let Some(rssi) = rssi_anon_rcv.try_changed() {
            let signal_above_squelch = rssi.dbm >= squelch_threshold_dbm;
            if signal_above_squelch {
                squelch_closed_since = None;
            } else if squelch_closed_since.is_none() {
                squelch_closed_since = Some(Instant::now());
            }
        }

        let squelch_closed_long_enough = squelch_closed_since
            .map(|since| since.elapsed() >= Duration::from_millis(SQUELCH_TRACK_WINDOW_MS))
            .unwrap_or(false);

        if !squelch_closed_long_enough {
            Timer::after_millis(LISTENING_WINDOW_MS).await;
            continue;
        }

        SWEEP_ACTIVE.sender().send(true);

        let deadline = Instant::now() + Duration::from_millis(FULL_LINE_BUDGET_MS);

        while !sweeper.is_line_complete() && Instant::now() < deadline {
            let freq = sweeper.next_bin_frequency(vfo_freq);
            SWEEP_REQUEST.sender().send(SweepRequest::SetFrequency(freq));
            let rssi = rssi_rcv.changed().await;
            sweeper.store_rssi(rssi.dbm);
        }

        SWEEP_REQUEST.sender().send(SweepRequest::Done);
        SWEEP_ACTIVE.sender().send(false);

        if sweeper.is_line_complete() {
            let line = sweeper.take_line();
            buffer.push(line);
            WATERFALL_LINE.sender().send(line);
        }

        Timer::after_millis(LISTENING_WINDOW_MS).await;
    }
}
