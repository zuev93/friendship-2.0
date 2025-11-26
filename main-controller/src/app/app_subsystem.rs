use embassy_executor::Spawner;

use crate::app::tasks::audio_task;

use super::tasks::{
    buttons_task::buttons_task, encoder_task::encoder_task, potentiometer_task::potentiometer_task,
};

pub struct AppSubsystem {}

impl AppSubsystem {
    pub fn init_subsystem(spawner: Spawner) {
        spawner.must_spawn(buttons_task());
        spawner.must_spawn(encoder_task());
        spawner.must_spawn(potentiometer_task());
        audio_task::spawn_tasks(spawner);
    }
}
