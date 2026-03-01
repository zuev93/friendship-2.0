use druzhba_macros::instrumented;
use crate::runtime_stats::TaskId;
use embassy_futures::select::{select, Either};
use embassy_time::{Duration, Instant, Timer, with_timeout};

use crate::app::{
    events::{BAND, FREQUENCY, MODE, SCAN_ACTIVE, SCAN_ENABLED, SCAN_RESUME, SCAN_STEP, SQUELCH},
    tasks::arbiters::frequency::{FrequencyCommand, FREQUENCY_CMD},
    types::{Band, Frequency, Mode},
};
use crate::main_board::events::CURRENT_RSSI2;

const DWELL_MS: u64 = 80;

enum ScanState {
    Idle,
    Scanning,
    Stopped,
}

fn squelch_to_dbm(raw: i16) -> i8 {
    const DBM_MIN: i32 = -120;
    const DBM_MAX: i32 = -20;
    const RAW_MAX: i32 = 1000;
    if raw <= 0 {
        return -128;
    }
    (DBM_MIN + (raw as i32 * (DBM_MAX - DBM_MIN) / RAW_MAX)) as i8
}

#[instrumented(TaskId::Scan)]
#[embassy_executor::task]
pub async fn scan_task() {
    let mut scan_enabled_rcv = SCAN_ENABLED.receiver().unwrap();
    let mut scan_step_rcv = SCAN_STEP.anon_receiver();
    let mut scan_resume_rcv = SCAN_RESUME.anon_receiver();
    let mut rssi_rcv = CURRENT_RSSI2.receiver().unwrap();
    let mut squelch_rcv = SQUELCH.anon_receiver();
    let mut band_rcv = BAND.anon_receiver();
    let mut mode_rcv = MODE.anon_receiver();

    let mut state = ScanState::Idle;
    let mut step_hz: u32 = 1000;
    let mut resume_secs: u64 = 3;
    let mut squelch_dbm: i8 = -128;
    let mut current_band = Band::Band20m;
    let mut freq: Frequency = 14_200_000;
    let mut signal_lost_at: Option<Instant> = None;

    loop {
        if let Some(s) = scan_step_rcv.try_changed() {
            step_hz = s.hz();
        }
        if let Some(r) = scan_resume_rcv.try_changed() {
            resume_secs = r.secs();
        }
        if let Some(sq) = squelch_rcv.try_changed() {
            squelch_dbm = squelch_to_dbm(sq.raw());
        }
        if let Some(b) = band_rcv.try_changed() {
            current_band = b;
        }

        match state {
            ScanState::Idle => {
                SCAN_ACTIVE.sender().send(false);
                loop {
                    let enabled = scan_enabled_rcv.changed().await;
                    if enabled {
                        if let Some(b) = band_rcv.try_changed() {
                            current_band = b;
                        }
                        if let Some(sq) = squelch_rcv.try_changed() {
                            squelch_dbm = squelch_to_dbm(sq.raw());
                        }
                        freq = FREQUENCY.try_get().unwrap_or(current_band.lower_frequency());
                        state = ScanState::Scanning;
                        SCAN_ACTIVE.sender().send(true);
                        break;
                    }
                }
            }
            ScanState::Scanning => {
                freq += step_hz;
                if freq > current_band.upper_frequency() {
                    freq = current_band.lower_frequency();
                }

                FREQUENCY_CMD.signal(FrequencyCommand::SetAbsolute(freq));

                match select(
                    Timer::after(Duration::from_millis(DWELL_MS)),
                    scan_enabled_rcv.changed(),
                )
                .await
                {
                    Either::First(_) => {}
                    Either::Second(enabled) => {
                        if !enabled {
                            state = ScanState::Idle;
                            continue;
                        }
                    }
                }

                if let Some(mode) = mode_rcv.try_changed() {
                    if mode == Mode::Tx {
                        state = ScanState::Idle;
                        continue;
                    }
                }

                if let Some(false) = scan_enabled_rcv.try_changed() {
                    state = ScanState::Idle;
                    continue;
                }

                let rssi_dbm = match with_timeout(
                    Duration::from_millis(50),
                    rssi_rcv.changed(),
                )
                .await
                {
                    Ok(rssi) => rssi.dbm,
                    Err(_) => -128,
                };

                if rssi_dbm >= squelch_dbm {
                    state = ScanState::Stopped;
                    signal_lost_at = None;
                }
            }
            ScanState::Stopped => {
                match select(
                    Timer::after(Duration::from_millis(200)),
                    scan_enabled_rcv.changed(),
                )
                .await
                {
                    Either::First(_) => {}
                    Either::Second(enabled) => {
                        if !enabled {
                            state = ScanState::Idle;
                            continue;
                        }
                    }
                }

                if let Some(rssi) = rssi_rcv.try_changed() {
                    if rssi.dbm >= squelch_dbm {
                        signal_lost_at = None;
                    } else if signal_lost_at.is_none() {
                        signal_lost_at = Some(Instant::now());
                    }
                }

                if let Some(lost_at) = signal_lost_at {
                    if lost_at.elapsed() >= Duration::from_secs(resume_secs) {
                        state = ScanState::Scanning;
                        signal_lost_at = None;
                    }
                }

                if let Some(mode) = mode_rcv.try_changed() {
                    if mode == Mode::Tx {
                        state = ScanState::Idle;
                        continue;
                    }
                }
            }
        }
    }
}
