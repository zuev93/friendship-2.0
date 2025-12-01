use embassy_stm32::exti::ExtiInput;

use crate::front_panel::{
    config::{default_button_mapping, default_potentiometer_mapping},
    events::{
        BandEncoderRotateEvent, ButtonEvent, PotentiometerEvent, VfoEncoderRotateEvent,
        BAND_ENCODER_EVENTS, BUTTON_EVENTS, HEADPHONES_CONNECTED, POTENTIOMETER_EVENTS,
        VFO_ENCODER_EVENTS,
    },
    types::ControlBusType,
};

use common::protocol_types::{
    ButtonEvent as ProtocolButtonEvent, ButtonState, EncoderDirection,
    EncoderEvent as ProtocolEncoderEvent, HeadphonesEvent as ProtocolHeadphonesEvent,
    PotentiometerValue,
};
use common::spi_protocol::PacketType;

pub async fn handle_response_packet(packet: &common::spi_protocol::Packet) {
    match packet.packet_type() {
        Some(PacketType::ButtonEvent) => {
            if let Some(event) = ProtocolButtonEvent::deserialize(packet) {
                handle_button_event(event).await;
            }
        }
        Some(PacketType::EncoderEvent) => {
            if let Some(event) = ProtocolEncoderEvent::deserialize(packet) {
                handle_encoder_event(event).await;
            }
        }
        Some(PacketType::PotentiometerValue) => {
            if let Some(value) = PotentiometerValue::deserialize(packet) {
                handle_potentiometer_value(value).await;
            }
        }
        Some(PacketType::HeadphonesEvent) => {
            if let Some(event) = ProtocolHeadphonesEvent::deserialize(packet) {
                handle_headphones_event(event).await;
            }
        }
        _ => {}
    }
}

// TODO check me
// seems to be flaky since Spi has start method and can listen by itself
#[embassy_executor::task]
pub async fn spi_receiver_task(control_bus: ControlBusType, mut alert_pin: ExtiInput<'static>) {
    loop {
        alert_pin.wait_for_low().await;

        let mut spi = control_bus.lock().await;

        if !alert_pin.is_low() {
            continue;
        }
        let packet = spi.receive_packet().await;

        if let Ok(packet) = packet {
            if packet.packet_type() != Some(PacketType::Idle) {
                handle_response_packet(&packet).await;
            }
        }
    }
}

async fn handle_button_event(event: ProtocolButtonEvent) {
    let mapping = default_button_mapping();

    let button_func = match mapping.get(event.id) {
        Some(func) => func,
        None => return,
    };

    let app_event = match event.state {
        ButtonState::Pressed => ButtonEvent::Press(button_func),
        ButtonState::Released => ButtonEvent::Release(button_func),
    };

    let _ = BUTTON_EVENTS.send(app_event).await;
}

async fn handle_encoder_event(event: ProtocolEncoderEvent) {
    let delta = match event.direction {
        EncoderDirection::Clockwise => event.steps,
        EncoderDirection::CounterClockwise => -event.steps,
    };

    match event.id {
        0 => {
            let _ = BAND_ENCODER_EVENTS
                .send(BandEncoderRotateEvent { delta })
                .await;
        }
        1 => {
            let _ = VFO_ENCODER_EVENTS
                .send(VfoEncoderRotateEvent { delta })
                .await;
        }
        _ => {}
    }
}

async fn handle_potentiometer_value(value: PotentiometerValue) {
    let mapping = default_potentiometer_mapping();

    let pot_func = match mapping.get(value.id) {
        Some(func) => func,
        None => return,
    };

    let event = PotentiometerEvent {
        function: pot_func,
        value: value.value as i16,
    };

    let _ = POTENTIOMETER_EVENTS.send(event).await;
}

async fn handle_headphones_event(event: ProtocolHeadphonesEvent) {
    HEADPHONES_CONNECTED.signal(event.connected);
}
