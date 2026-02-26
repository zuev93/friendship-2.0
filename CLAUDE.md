# Friendship 2.0 - Druzhba-M Modern Transceiver

## Project Context
Modern digital redesign of the classic Soviet "Druzhba-M" HF transceiver. Superheterodyne architecture preserved, analog components replaced with digital equivalents. Firmware written in Rust (embedded, no_std, async) using the Embassy framework.

## Architecture
Distributed system with two STM32 microcontrollers communicating via SPI:
- **Main Controller**: STM32H563VI - core transceiver logic, signal processing, peripheral control
- **Front Panel Controller**: STM32H563VI - user interface, displays, encoders, buttons, audio output

## Code Structure
```
common/src/drivers/       - Hardware abstraction drivers (shared between controllers)
main-controller/src/
  main_board/modules/     - Mixer, Detector, IF Amp, Crystal Filter, Audio Panel
  peripherals/modules/    - BPF, LPF, HF Amp (external boards on I2C4)
  control_board/modules/  - Power control, Audio (I2S on SPI6)
  front_panel/            - SPI link to front panel controller
  i2c_map.rs              - All I2C address definitions (single source of truth)
  hardware.rs             - Hardware initialization and pin assignments
front-panel-controller/src/
  hardware/               - Button, encoder, display, WM8940, LED, potentiometer drivers
  state/                  - Input/output state management
  tasks/                  - Async task handlers
```

## I2C Bus Layout
- **I2C1** (Main Board, 400kHz async): Mixer, Crystal Filter, IF Amp, Detector, Audio Panel
- **I2C3** (Control Board, blocking): Power monitoring (INA3221)
- **I2C4** (Peripherals, 400kHz async+DMA): BPF, LPF, HF Amp
- **I2C1** (Front Panel, 100kHz): WM8940 audio codec

## Rules
1. User is the ONLY architect. Never make architectural decisions independently
2. Never add comments to code unless explicitly requested
3. Never use unsafe code unless explicitly approved
4. Never use #[allow(dead_code)] or any other allow attributes - fix the code instead
5. Always check existing codebase, Cargo.toml, hardware.rs, datasheets BEFORE asking
6. Read documentation and search web for technical questions about chips/peripherals
7. When stuck - STOP and ASK, never assume
8. Report problems immediately, don't work around them
9. Write clean, working code following instructions exactly
10. **NEVER modify pin assignments without asking the user first** - pins are hardware. PCBs are physical and cannot be changed after manufacturing. Be extra cautious about any hardware-related changes
