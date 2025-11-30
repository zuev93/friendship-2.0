use crate::{
    front_panel::{tasks::spi_receiver::handle_response_packet, types::ControlBusType},
    main_board::events::CURRENT_RSSI,
};
use common::protocol_types::SMeterCommand;
use common::spi_protocol::Packet;

#[embassy_executor::task]
pub async fn s_meter_task(control_bus: ControlBusType) {
    loop {
        let rssi = CURRENT_RSSI.wait().await;
        let value = dbm_to_s_meter_value(rssi.dbm);

        let smeter_cmd = SMeterCommand { value };
        let mut packet = Packet::new();
        smeter_cmd.serialize(&mut packet);

        let response = {
            let mut spi = control_bus.lock().await;
            spi.send_packet(&packet).await
        };

        if let Ok(response_packet) = response {
            handle_response_packet(&response_packet).await;
        }
    }
}

fn dbm_to_s_meter_value(dbm: i8) -> u16 {
    const DBM_MIN: i32 = -120;
    const DBM_MAX: i32 = -20;
    const DAC_MAX: u16 = 4095;
    const DBM_RANGE: i32 = DBM_MAX - DBM_MIN;

    let dbm_clamped = dbm.clamp(DBM_MIN as i8, DBM_MAX as i8) as i32;
    let normalized = ((dbm_clamped - DBM_MIN) * DAC_MAX as i32) / DBM_RANGE;

    normalized.max(0).min(DAC_MAX as i32) as u16
}
