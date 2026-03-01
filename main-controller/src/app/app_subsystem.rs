use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use static_cell::StaticCell;

use crate::app::{tasks::audio_task, tone_generator::ToneGenerator};

use super::tasks::{
    alc::alc_task,
    arbiters::{
        clarifier_mode::clarifier_mode_arbiter_task, clarifier_value::clarifier_value_arbiter_task,
        filter::filter_arbiter_task, frequency::frequency_arbiter_task,
        if_gain::if_gain_arbiter_task, if_gain_mode::if_gain_mode_arbiter_task,
        microphone::microphone_arbiter_task, mode::mode_arbiter_task, nb::nb_arbiter_task,
        nb_level::nb_level_arbiter_task, rf_gain_mode::rf_gain_mode_arbiter_task,
        rf_power::rf_power_arbiter_task, squelch::squelch_arbiter_task,
        tone::tone_arbiter_task, transmit_mode::transmit_mode_arbiter_task,
        volume::volume_arbiter_task,
    },
    buttons_task::buttons_task,
    encoder_task::encoder_task,
    sweep_scheduler::sweep_scheduler_task,
    tone_task,
};

pub struct AppSubsystem {}

impl AppSubsystem {
    pub fn init_subsystem(spawner: Spawner) {
        static TONE_GENERATOR: StaticCell<Mutex<ThreadModeRawMutex, ToneGenerator>> =
            StaticCell::new();
        let mutex = TONE_GENERATOR.init(Mutex::new(ToneGenerator::new()));

        spawner.must_spawn(alc_task());
        spawner.must_spawn(buttons_task());
        spawner.must_spawn(encoder_task());
        spawner.must_spawn(sweep_scheduler_task());

        spawner.must_spawn(mode_arbiter_task());
        spawner.must_spawn(tone_arbiter_task());
        spawner.must_spawn(transmit_mode_arbiter_task());
        spawner.must_spawn(filter_arbiter_task());
        spawner.must_spawn(frequency_arbiter_task());
        spawner.must_spawn(if_gain_mode_arbiter_task());
        spawner.must_spawn(rf_gain_mode_arbiter_task());
        spawner.must_spawn(clarifier_mode_arbiter_task());
        spawner.must_spawn(nb_arbiter_task());

        spawner.must_spawn(volume_arbiter_task());
        spawner.must_spawn(microphone_arbiter_task());
        spawner.must_spawn(rf_power_arbiter_task());
        spawner.must_spawn(if_gain_arbiter_task());
        spawner.must_spawn(clarifier_value_arbiter_task());
        spawner.must_spawn(squelch_arbiter_task());
        spawner.must_spawn(nb_level_arbiter_task());

        tone_task::spawn(spawner, mutex);
        audio_task::spawn_tasks(spawner, mutex);
    }
}
