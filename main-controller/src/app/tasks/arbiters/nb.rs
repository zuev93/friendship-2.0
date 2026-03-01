use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;

use crate::app::events::NB_ENABLED;

pub enum NbCommand {
    Toggle,
}

pub static NB_CMD: Signal<ThreadModeRawMutex, NbCommand> = Signal::new();

#[embassy_executor::task]
pub async fn nb_arbiter_task() {
    let mut nb_enabled = false;

    loop {
        let cmd = NB_CMD.wait().await;
        match cmd {
            NbCommand::Toggle => {
                nb_enabled = !nb_enabled;
                NB_ENABLED.signal(nb_enabled);
            }
        }
    }
}
