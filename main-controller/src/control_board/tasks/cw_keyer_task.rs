use embassy_executor::Spawner;
use embassy_time::Timer;

use crate::app::cw_keyer::CwKeyer;
use crate::app::events::{
    CW_BREAK_IN_DELAY, CW_KEY_MODE, CW_PADDLE_SWAP, CW_SIDETONE_ACTIVE, CW_WEIGHT, CW_WPM,
    TRANSMIT_MODE,
};
use crate::app::tasks::arbiters::mode::{ModeCommand, MODE_CMD};
use crate::app::types::TransmitMode;
use crate::control_board::modules::cw_paddle::CwPaddle;
use crate::runtime_stats::TaskId;
use druzhba_macros::instrumented;

struct PaddleDebounce {
    dit_count: u8,
    dah_count: u8,
    dit_stable: bool,
    dah_stable: bool,
}

impl PaddleDebounce {
    fn new() -> Self {
        Self {
            dit_count: 0,
            dah_count: 0,
            dit_stable: false,
            dah_stable: false,
        }
    }

    fn update(&mut self, dit_raw: bool, dah_raw: bool) {
        if dit_raw == self.dit_stable {
            self.dit_count = 0;
        } else {
            self.dit_count += 1;
            if self.dit_count >= 2 {
                self.dit_stable = dit_raw;
                self.dit_count = 0;
            }
        }
        if dah_raw == self.dah_stable {
            self.dah_count = 0;
        } else {
            self.dah_count += 1;
            if self.dah_count >= 2 {
                self.dah_stable = dah_raw;
                self.dah_count = 0;
            }
        }
    }
}

pub fn create_task(spawner: Spawner, paddle: CwPaddle) {
    spawner.must_spawn(cw_keyer_task(paddle));
}

#[instrumented(TaskId::CwKeyer)]
#[embassy_executor::task]
async fn cw_keyer_task(paddle: CwPaddle) {
    let mut keyer = CwKeyer::new();
    let mut debounce = PaddleDebounce::new();
    let mut swap = false;

    let mut mode_rcv = CW_KEY_MODE.receiver().unwrap();
    let mut wpm_rcv = CW_WPM.receiver().unwrap();
    let mut weight_rcv = CW_WEIGHT.receiver().unwrap();
    let mut swap_rcv = CW_PADDLE_SWAP.receiver().unwrap();
    let mut delay_rcv = CW_BREAK_IN_DELAY.receiver().unwrap();
    let mut tx_mode_rcv = TRANSMIT_MODE.receiver().unwrap();

    let mut cw_active = false;

    loop {
        Timer::after_millis(1).await;

        if let Some(m) = mode_rcv.try_changed() {
            keyer.set_mode(m);
        }
        if let Some(w) = wpm_rcv.try_changed() {
            keyer.set_wpm(w.raw());
        }
        if let Some(w) = weight_rcv.try_changed() {
            keyer.set_weight(w.raw());
        }
        if let Some(s) = swap_rcv.try_changed() {
            swap = s;
        }
        if let Some(d) = delay_rcv.try_changed() {
            keyer.set_hang_ticks(d.ms());
        }
        if let Some(tm) = tx_mode_rcv.try_changed() {
            cw_active = tm == TransmitMode::Cw;
        }

        if !cw_active {
            continue;
        }

        let dit_raw = paddle.dit_pressed();
        let dah_raw = paddle.dah_pressed();

        let (dit_in, dah_in) = if swap {
            (dah_raw, dit_raw)
        } else {
            (dit_raw, dah_raw)
        };

        debounce.update(dit_in, dah_in);

        let output = keyer.tick(debounce.dit_stable, debounce.dah_stable);

        if let Some(key_on) = output.key_changed {
            CW_SIDETONE_ACTIVE.sender().send(key_on);
        }
        if let Some(tx_on) = output.tx_changed {
            if tx_on {
                MODE_CMD.signal(ModeCommand::CwKeyDown);
            } else {
                MODE_CMD.signal(ModeCommand::CwKeyUp);
            }
        }
    }
}
