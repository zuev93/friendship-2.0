use embassy_executor::Spawner;

use crate::control_board::modules::usb::Usb;
use crate::control_board::usb::tasks::{
    cat_task::cat_task, cli_task::cli_task, speaker_task::speaker_task,
    usb_device_task::usb_device_task,
};

pub fn create_tasks(spawner: Spawner, usb: Usb) {
    spawner.must_spawn(usb_device_task(usb.device));
    spawner.must_spawn(cat_task(usb.cat_cdc));
    spawner.must_spawn(cli_task(usb.cli_cdc));
    spawner.must_spawn(speaker_task(usb.stream, usb.feedback, usb.control_monitor));
}
