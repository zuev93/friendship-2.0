use embassy_stm32::exti::ExtiInput;

use crate::front_panel::{
    config::{default_button_mapping, default_encoder_mapping},
    events::{
        ButtonEvent, EncoderEvent, BUTTON_EVENTS, ENCODER_EVENTS, HEADPHONES_CONNECTED,
        MENU_ENCODER_EVENTS,
    },
    types::{ControlBusType, EncoderFunction},
};
use crate::runtime_stats::TaskId;
use druzhba_macros::instrumented;

use crate::app::events::DISPLAY_FPS;
use crate::app::types::DisplayFps;
use common::protocol_types::{
    ButtonEvent as ProtocolButtonEvent, ButtonState, DisplayFpsEvent, EncoderDirection,
    EncoderEvent as ProtocolEncoderEvent, HeadphonesEvent as ProtocolHeadphonesEvent,
};
use common::spi_protocol::{PacketSerializable, PacketType};

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
        Some(PacketType::DisplayFpsEvent) => {
            if let Some(event) = DisplayFpsEvent::deserialize(packet) {
                DISPLAY_FPS.sender().send(DisplayFps::new(event.fps));
            }
        }
        _ => {}
    }
}

#[instrumented(TaskId::SpiReceiver)]
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
        EncoderFunction::Menu => {
            MENU_ENCODER_EVENTS.send(delta).await;
        }
        _ => {
            ENCODER_EVENTS
                .send(EncoderEvent {
                    function,
                    delta,
                })
                .await;
        }
    }
}

async fn handle_headphones_event(event: ProtocolHeadphonesEvent) {
    HEADPHONES_CONNECTED.sender().send(event.connected);
}
