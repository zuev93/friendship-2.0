# Druzhba-M Transceiver (Modern Rust Implementation) 🦀📻

A modern digital redesign of the classic Soviet "Druzhba-M" transceiver, implemented in Rust with contemporary hardware while preserving the superheterodyne architecture.

**Reference Documentation**: Original Druzhba-M schematic and technical details available at [CQHAM.ru](https://www.cqham.ru/druzba-m.htm)

## Project Overview

This project aims to create a modern version of the legendary "Druzhba-M" transceiver - a Soviet-era shortwave radio transceiver known for its reliability and performance. The goal is to maintain the proven superheterodyne design principles while replacing outdated components with modern digital alternatives.

**⚠️ Work in Progress**: This project is actively under development. Schematics and comprehensive documentation will be shared as development progresses.

## Key Modernizations

### Digital User Interface
- **Graphic OLED displays** (SSD1315): Replacing 7-segment indicators with modern graphical displays for rich user interfaces
- **Tactile buttons with I2C bridges**: Digital button matrix controlled by microcontrollers instead of mechanical switches
- **Precision rotary encoders**: Digital potentiometers replacing variable capacitors and mechanical tuning controls

### Digital Audio Processing
- **Digital audio codec** (WM8940): Complete digital audio processing pipeline replacing analog audio circuits
- **Digital audio mixer**: Software-controlled audio mixing and routing
- **Digital filtering**: DSP-based audio filtering for improved signal quality

### Frequency Synthesis & Signal Processing
- **DDS synthesizers** (AD9834): Crystal-based digital frequency synthesis replacing traditional VFO circuits
- **Digital ADC/DAC** (ADS1115, MCP4725): High-precision analog-to-digital conversion for measurements and control
- **Digital filters**: Software-defined bandpass (BPF) and lowpass (LPF) filters

### Power Management
- **Digital power monitoring** (INA3221): Real-time voltage and current monitoring for all subsystems
- **Smart power control**: Microcontroller-managed power sequencing and protection

### Hardware Architecture
The system consists of three STM32 microcontrollers communicating via SPI and I2C buses:

- **Front Panel Controller** (STM32F407): User interface, OLED displays, buttons, encoders, S-meter, audio output
- **Main Controller** (STM32G474): Core transceiver logic, signal processing, modulation, frequency synthesis
- **Control Board Controller**: Power management, audio codec control, system monitoring

### Communication Infrastructure
- **SPI protocol**: High-speed communication between main controllers
- **I2C network**: Peripheral device management with bridge chips (SC18IS602, PCA9534, TCA9554)
- **Event-driven architecture**: Asynchronous processing with Embassy framework

## Technology Stack

- **Language**: Rust 🦀 (embedded, no_std, async)
- **Framework**: Embassy for async embedded development
- **Hardware**: STM32F407VE and STM32G474RE microcontrollers
- **Communication**: SPI and I2C protocols for inter-board communication
- **Architecture**: Event-driven embedded systems with message passing
- **Build System**: Cargo with custom target configuration
- **Development Tools**: Probe-rs for debugging, cargo-embed for flashing

## Features (Implemented & Planned)

### ✅ Implemented
- Digital frequency synthesis using DDS (AD9834)
- Graphic OLED displays (SSD1315) replacing 7-segment indicators
- Digital audio processing with WM8940 codec
- Rotary encoders for precise tuning control
- Digital button matrix with I2C expanders
- SPI/I2C communication infrastructure
- Event-driven embedded architecture
- Digital power monitoring and control

### 🚧 In Development
- Software-defined filtering (BPF/LPF)
- Digital modulation/demodulation
- Advanced DSP audio processing
- USB connectivity for computer control
- Touchscreen interface capabilities
- Remote control functionality

### 📋 Planned
- Improved receiver sensitivity and selectivity
- Multiple operating modes (SSB, CW, AM)
- Digital signal strength metering
- Automatic gain control (AGC)
- Noise reduction algorithms
- Frequency hopping capabilities

## Getting Started

```bash
# Clone the repository
git clone https://github.com/zuev93/friendship-2.0.git
cd friendship-2.0

# Build the project
cargo build --release
```

## Project Structure

- `common/` - Shared Rust library with hardware abstractions and device drivers
- `front-panel-controller/` - STM32F407 firmware for user interface (displays, buttons, encoders)
- `main-controller/` - STM32G474 firmware for core transceiver functionality

### Architecture Details

The system uses a distributed architecture with two STM32 microcontrollers:

#### Main Controller (STM32G474)
- **app/**: High-level application logic, audio processing, tone generation
- **main_board/**: Core transceiver modules (mixer, detector, IF amplifier, DDS control)
- **control_board/**: Power management, audio codec control
- **front_panel/**: Communication with front panel controller
- **peripherals/**: External devices (filters, amplifiers)
- **display/**: System status display management

#### Front Panel Controller (STM32F407)
- **hardware/**: Button, encoder, display, and LED drivers
- **state/**: Input processing and output state management
- **tasks/**: Asynchronous task handlers for UI elements

#### Common Library
- **drivers/**: Hardware abstraction layer for all peripherals
- **spi_protocol/**: Inter-controller communication protocol
- **protocol_types/**: Shared data structures and message types

## Contributing

This project welcomes contributions! Please feel free to open issues or submit pull requests.

## License

TBD - License information will be added as the project matures.

---

*Inspired by the classic "Druzhba-M" transceiver, modernized for the 21st century.*
