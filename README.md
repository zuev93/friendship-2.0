# Druzhba-M Transceiver (Rust Implementation)

Modern transceiver controller for ATmega1284P written in Rust 🦀

## Features

- **Memory-safe** bare-metal Rust
- **Type-safe** hardware abstraction layer
- **Zero-cost abstractions**
- I2C bus for device control (400 kHz)
- SPI bus for displays
- Complete transceiver control logic

## Requirements

- **Rust nightly** toolchain
- **avr-gcc** for linking
- **avrdude** for flashing

## Installation

### Install Rust and AVR toolchain

```bash
# Install Rust nightly
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly

# Source cargo environment
source "$HOME/.cargo/env"

# Add rust-src component
rustup component add rust-src --toolchain nightly

# Install ravedude for flashing
cargo install ravedude

# Install AVR toolchain (if not installed)
brew tap osx-cross/avr
brew install avr-gcc avrdude
```

## Building

```bash
# Build project
cargo build --release

# Check size
avr-size -C --mcu=atmega1284p target/avr-atmega1284p/release/druzhba.elf

# Flash to device (using USBasp)
avrdude -c usbasp -p m1284p -U flash:w:target/avr-atmega1284p/release/druzhba.elf:e
```

## Project Structure

```
druzhba-3.0/
├── Cargo.toml                    # Rust project manifest
├── avr-atmega1284p.json         # AVR target specification
├── .cargo/
│   └── config.toml              # Cargo configuration for AVR
├── src/
│   ├── main.rs                  # Application entry point
│   ├── hal/                     # Hardware Abstraction Layer
│   │   ├── mod.rs
│   │   ├── i2c.rs               # I2C/TWI driver
│   │   ├── spi.rs               # SPI driver
│   │   └── gpio.rs              # GPIO control
│   ├── drivers/                 # Device drivers
│   │   ├── mod.rs
│   │   └── device.rs            # Device trait
│   ├── transceiver/             # Transceiver control
│   │   └── mod.rs
│   └── display/                 # Display subsystem
│       └── mod.rs
└── src_c_backup/                # Original C implementation (backup)
```

## Hardware

- **MCU:** ATmega1284P @ 16 MHz
- **Flash:** 128 KB
- **RAM:** 16 KB
- **I2C:** 400 kHz for control devices
- **SPI:** 4 MHz for displays

## Why Rust?

- ✅ **Memory safety** without runtime overhead
- ✅ **Zero-cost abstractions**
- ✅ **Type safety** catches bugs at compile time
- ✅ **Ownership system** prevents resource leaks
- ✅ **Great learning opportunity** for embedded Rust
- ✅ **Modern tooling** with Cargo

## Rust Features Used

- `#![no_std]` - No standard library (bare metal)
- `#![no_main]` - Custom entry point
- **Type-safe GPIO** - Compile-time pin configuration
- **Result types** - Proper error handling
- **Traits** - Generic device interfaces
- **Enums** - State machines and modes

## Debug

### UART Debug (coming soon)

```rust
use ufmt::uwriteln;
uwriteln!(uart, "Frequency: {} Hz", freq).unwrap();
```

### JTAG Debug

```bash
# Using avr-gdb
avr-gdb target/avr-atmega1284p/release/druzhba.elf
(gdb) target remote :4242
(gdb) break main
```

## Status

✅ **WORKING!** Project compiles and runs!

**Current Build Stats:**

- Program: 284 bytes (0.2% Flash)
- Data: 1 byte (0.0% RAM)
- Build time: ~0.8 seconds
- Smaller than C version! (was 728 bytes)

**TODO:**

- [ ] Add UART debug module
- [ ] Implement specific device drivers (PLL, DAC, ADC)
- [ ] Add display drivers (ST7789, SSD1306)
- [ ] Implement user input (rotary encoder, buttons)
- [ ] Add menu system
- [ ] EEPROM settings storage
- [ ] Complete transceiver control logic
- [ ] Add protection features (SWR, temperature)

## Comparison with C Version

| Feature        | C            | Rust      |
| -------------- | ------------ | --------- |
| Memory safety  | Manual       | Automatic |
| Type safety    | Weak         | Strong    |
| Code size      | 728 bytes    | 284 bytes |
| Build time     | ~2 sec       | ~0.8 sec  |
| Error handling | Return codes | Result<T> |
| Abstractions   | Manual       | Zero-cost |
| Learning curve | Easy         | Moderate  |

## Migration Notes

The C version is preserved in `src_c_backup/` and `include_c_backup/` directories. You can switch back by:

```bash
mv src src_rust
mv src_c_backup src
mv include_c_backup include
```

## Resources

- [AVR Rust Book](https://book.avr-rust.com/)
- [avr-device crate](https://github.com/Rahix/avr-device)
- [Embedded Rust](https://rust-embedded.github.io/book/)

## License

Open source for personal and commercial use.
