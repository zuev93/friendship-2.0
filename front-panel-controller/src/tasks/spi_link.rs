use druzhba_common::error;
use druzhba_common::protocol_types::{
    LedCommand, MenuCommand, RadioStateCommand, WaterfallLineCommand, Wm8940Command,
};
use druzhba_common::spi_protocol::{Crc16, Packet, PacketSerializable, PacketType};
use embassy_executor::Spawner;
use embassy_stm32::gpio::Output;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;

use crate::constants::TX_QUEUE_SIZE;
use crate::crc::HardwareCrc16Modbus;
use crate::hardware::{SpiLink, SpiSlaveInstance};
use crate::state::input::InputState;
use crate::state::menu::MENU_EVENTS;
use crate::state::output::OUTPUT_EVENTS;

static TX_QUEUE: Channel<ThreadModeRawMutex, Packet, TX_QUEUE_SIZE> = Channel::new();
static QUEUE_EMPTY: Signal<ThreadModeRawMutex, ()> = Signal::new();

pub fn spawn_tasks(
    spawner: &Spawner,
    spi_link: SpiLink,
    input_state: &'static InputState,
    crc: &'static HardwareCrc16Modbus,
) {
    let SpiLink { spi, link_alert } = spi_link;

    spawner.must_spawn(prepare_tx_task(link_alert, crc));
    spawner.must_spawn(spi_link_task(spi, input_state, crc));
}

#[embassy_executor::task]
async fn prepare_tx_task(mut alert: Output<'static>, crc: &'static HardwareCrc16Modbus) {
    use embassy_futures::select::{select, Either};

    loop {
        match select(OUTPUT_EVENTS.receive(), QUEUE_EMPTY.wait()).await {
            Either::First(event) => {
                let mut packet = Packet::new();
                serialize_event(&event, &mut packet, crc);
                TX_QUEUE.send(packet).await;
                alert.set_high();
            }
            Either::Second(_) => {
                alert.set_low();
            }
        }
    }
}

#[embassy_executor::task]
async fn spi_link_task(
    mut spi: SpiSlaveInstance,
    input_state: &'static InputState,
    crc: &'static HardwareCrc16Modbus,
) {
    let mut rx_packet = Packet::new();
    let mut idle_packet = Packet::new();
    idle_packet.set_type(PacketType::Idle);
    idle_packet.set_crc(crc);

    loop {
        let tx_data = match TX_QUEUE.try_receive() {
            Ok(packet) => packet.data,
            Err(_) => {
                QUEUE_EMPTY.signal(());
                idle_packet.data
            }
        };

        if let Err(_e) = spi.transfer(&mut rx_packet.data, &tx_data).await {
            error::error("SPI transfer failed").await;
            continue;
        }

        if rx_packet.verify_crc(crc) {
            handle_rx_packet(&rx_packet, input_state).await;
        }
    }
}

async fn handle_rx_packet(packet: &Packet, input_state: &'static InputState) {
    if let Some(led_cmd) = LedCommand::deserialize(packet) {
        if led_cmd.led_id < 7 {
            input_state.leds.signal(crate::state::input::LedUpdate {
                led_id: led_cmd.led_id,
                state: led_cmd.state,
            });
        }
    } else if let Some(wm8940_cmd) = Wm8940Command::deserialize(packet) {
        input_state.wm8940.signal(wm8940_cmd);
    } else if let Some(cmd) = RadioStateCommand::deserialize(packet) {
        input_state.radio_state.signal(crate::state::input::RadioState {
            rssi_dbm: cmd.rssi_dbm,
            forward_power_mw: cmd.forward_power_mw,
            vswr_x100: cmd.vswr_x100,
            mode: cmd.mode,
            transmit_mode: cmd.transmit_mode,
            agc_mode: cmd.agc_mode,
            rf_gain_mode: cmd.rf_gain_mode,
            filter_bw_hz: cmd.filter_bw_hz,
            frequency: cmd.frequency,
            band: cmd.band,
            nb_enabled: cmd.nb_enabled,
            clarifier_mode: cmd.clarifier_mode,
            clarifier_raw: cmd.clarifier_raw,
            rf_power_centipercent: cmd.rf_power_centipercent,
            volume_raw: cmd.volume_raw,
            squelch_raw: cmd.squelch_raw,
            cursor_index: cmd.cursor_index,
            cursor_editing: cmd.cursor_editing,
        });
    } else if let Some(cmd) = WaterfallLineCommand::deserialize(packet) {
        input_state.waterfall_line.signal(crate::state::input::WaterfallLineData {
            center_freq: cmd.center_freq,
            span_hz: cmd.span_hz,
            sweep_status: cmd.sweep_status,
            bins: cmd.bins,
        });
    } else if let Some(cmd) = MenuCommand::deserialize(packet) {
        let _ = MENU_EVENTS.try_send(cmd);
    }
}

fn serialize_event(event: &crate::state::output::OutputEvent, packet: &mut Packet, crc: &impl Crc16) {
    use crate::state::output::OutputEvent;
    match event {
        OutputEvent::Button(btn) => btn.serialize(packet, crc),
        OutputEvent::Encoder(enc) => enc.serialize(packet, crc),
        OutputEvent::Headphones(hp) => hp.serialize(packet, crc),
    }
}
