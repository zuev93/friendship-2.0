use embassy_stm32::exti::ExtiInput;

use crate::front_panel::{
    config::{default_button_mapping, default_encoder_mapping},
    events::{
        BandEncoderRotateEvent, ButtonEvent, ControlEncoderEvent, VfoEncoderRotateEvent,
        BAND_ENCODER_EVENTS, BUTTON_EVENTS, CONTROL_ENCODER_EVENTS, HEADPHONES_CONNECTED,
        MENU_ENCODER_EVENTS, VFO_ENCODER_EVENTS,
    },
    types::{ControlBusType, EncoderFunction},
};

use common::protocol_types::{
    ButtonEvent as ProtocolButtonEvent, ButtonState, EncoderDirection,
    EncoderEvent as ProtocolEncoderEvent, HeadphonesEvent as ProtocolHeadphonesEvent,
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
        Some(PacketType::HeadphonesEvent) => {
            if let Some(event) = ProtocolHeadphonesEvent::deserialize(packet) {
                handle_headphones_event(event).await;
            }
        }
        _ => {}
    }
}

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
    let mapping = default_encoder_mapping();

    let function = match mapping.get(event.id) {
        Some(func) => func,
        None => return,
    };

    let delta = match event.direction {
        EncoderDirection::Clockwise => event.steps,
        EncoderDirection::CounterClockwise => -event.steps,
    };

    match function {
        EncoderFunction::Band => {
            BAND_ENCODER_EVENTS
                .send(BandEncoderRotateEvent { delta })
                .await;
        }
        EncoderFunction::Vfo => {
            VFO_ENCODER_EVENTS
                .send(VfoEncoderRotateEvent { delta })
                .await;
        }
        EncoderFunction::Menu => {
            MENU_ENCODER_EVENTS.send(delta).await;
        }
        func => {
            CONTROL_ENCODER_EVENTS
                .send(ControlEncoderEvent {
                    function: func,
                    delta,
                })
                .await;
        }
    }
}

async fn handle_headphones_event(event: ProtocolHeadphonesEvent) {
    HEADPHONES_CONNECTED.signal(event.connected);
}
