use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use static_cell::StaticCell;

use crate::app::{tasks::audio_task, tone_generator::ToneGenerator};

use super::tasks::{
    buttons_task::buttons_task, control_encoder_task::control_encoder_task,
    encoder_task::encoder_task, frequency_arbiter::frequency_arbiter_task,
    sweep_scheduler::sweep_scheduler_task, tone_task,
};

pub struct AppSubsystem {}

impl AppSubsystem {
    pub fn init_subsystem(spawner: Spawner) {
        static TONE_GENERATOR: StaticCell<Mutex<ThreadModeRawMutex, ToneGenerator>> =
            StaticCell::new();
        let mutex = TONE_GENERATOR.init(Mutex::new(ToneGenerator::new()));

        spawner.must_spawn(buttons_task());
        spawner.must_spawn(encoder_task());
        spawner.must_spawn(control_encoder_task());
        spawner.must_spawn(frequency_arbiter_task());
        spawner.must_spawn(sweep_scheduler_task());
        tone_task::spawn(spawner, mutex);
        audio_task::spawn_tasks(spawner, mutex);
    }
}
