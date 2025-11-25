/*
 * I2C Device Address Map
 *
 * Centralized I2C address management for all devices.
 * Helps prevent address conflicts and simplifies hardware changes.
 *
 * NOTE: System uses TWO separate I2C buses:
 * 1. Transceiver I2C bus - RF/IF control and measurement
 * 2. Front Panel I2C bus - User interface controls and indicators
 *
 * Addresses can overlap between buses without conflicts.
 */

pub const SC18IS602_DDS_ADDR: u8 = 0x28; // 7-bit: 0b0101000
pub const SC18IS602_IF_REF_ADDR: u8 = 0x29; // 7-bit: 0b0101001
pub const SC18IS602_TONE_GEN_ADDR: u8 = 0x2A; // 7-bit: 0b0101010
pub const MCP4725_TX_POWER_ADDR: u8 = 0x60; // A0 = GND
pub const MCP4725_IF_GAIN_ADDR: u8 = 0x61; // A0 = VDD
pub const ADS1115_RSSI_ADDR: u8 = 0x48; // 7-bit: 0b1001000 (ADDR = GND)
pub const PCM3060_AUDIO_PANEL_ADDR: u8 = 0x46; // 7-bit: 0b1000110
