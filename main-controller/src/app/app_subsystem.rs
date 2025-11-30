use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use static_cell::StaticCell;

use crate::app::{tasks::audio_task, tone_generator::ToneGenerator};

use super::tasks::{
    buttons_task::buttons_task, encoder_task::encoder_task, potentiometer_task::potentiometer_task,
    tone_task,
};

pub struct AppSubsystem {}

impl AppSubsystem {
    pub fn init_subsystem(spawner: Spawner) {
        static STATIC_CELL: StaticCell<Mutex<ThreadModeRawMutex, ToneGenerator>> =
            StaticCell::new();
        let mutex = STATIC_CELL.init(Mutex::new(ToneGenerator::new()));

        spawner.must_spawn(buttons_task());
        spawner.must_spawn(encoder_task());
        spawner.must_spawn(potentiometer_task());
        tone_task::spawn(spawner, mutex);
        audio_task::spawn_tasks(spawner, mutex);
    }
}
