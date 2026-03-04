# Druzhba-M 2.0 / Дружба-М 2.0 ("Friendship") — Modern HF Transceiver

An open-source HF transceiver designed to fit a single-DIN slot (180x50 mm — the size of a car stereo). The goal is a compact, fully featured radio that competes with commercial transceivers like the Icom IC-7300 or Yaesu FT-891 in feature set, while being fully open-source and buildable from readily available components.

Based on the classic Soviet "Druzhba-M" superheterodyne architecture — a proven RF topology — with all control and signal management brought into the digital domain. PLL synthesis (Si5351), digital codecs, software-controlled filtering, USB-C power, and real-time DSP — in a package you can mount in a dashboard.

**Original design**: V. Abramov (UX5PS) & S. Telezhnikov (RV3YF) — [Druzhba-M on CQHAM.ru](https://www.cqham.ru/druzba-m.htm)

## Why This Exists

I was building an original Druzhba-M and got tired of endless manual calibration — trimming VFOs, matching transistors, aligning filters. The radio itself sounds great, the architecture is solid, but the analog control side is tedious to get right. So I decided to keep the RF design and replace all the fiddly analog control with digital — make it repeatable, make it compact, make it the best I can.

Every board is a custom PCB designed in EasyEDA. Schematics and netlists are in the `hardware/` directory.

## Key Design Decisions

**Superheterodyne, not SDR.** This is intentionally not a software-defined radio. The signal path is a real superheterodyne chain — BPF, mixer (FST3125 H-mode), 10 MHz crystal filter, IF amp, product detector — with each stage on its own module board. Digital control means every parameter is software-adjustable, but the RF path is analog where it matters.

**Why microcontrollers at all?** Every module board has digitally controlled components — DDS chips, DACs, ADCs, GPIO expanders, relay drivers. Something needs to orchestrate all of that: set frequencies, switch bands, read RSSI, manage gain, monitor power, handle USB, run the UI. A microcontroller with I2C buses and async firmware replaces what would otherwise be a rats nest of discrete logic and manual controls.

**Two-controller split.** The main controller runs the signal path and power management. A separate front panel controller handles displays, buttons, encoders, and headphone audio. They communicate over a high-speed SPI link. This keeps display rendering and UI polling away from time-sensitive RF control, and lets each side be developed and debugged independently.

**I2C for module control.** All module boards connect to the main controller via I2C buses. GPIO expanders (PCA9534, TCA9555) switch relays for band filters. DACs (MCP4725) set gain and bias voltages. ADCs (ADS1015, ADS1115) read RSSI, power levels, and temperatures. The Si5351 PLL synthesizer generates both VFO and BFO clocks, also controlled over I2C. Three separate I2C buses keep traffic isolated: main signal path, power monitoring, and external peripherals (filters, PA).

**Hardware DSP.** The STM32H5 has a CORDIC math coprocessor and an FMAC FIR accelerator. All trigonometric, exponential, and logarithmic math runs on the CORDIC hardware — no lookup tables, no software approximations. Audio filtering uses the hardware FMAC.

**USB-C Power Delivery.** The transceiver is powered via USB-C PD. The STM32H5's built-in UCPD peripheral negotiates power contracts directly — no external PD controller needed. The PA gets its 50V from a boost converter enabled only during transmit.

**Rust, async, no_std.** Fully async firmware on the Embassy framework. No RTOS, no heap, no unsafe. Every peripheral interaction is non-blocking. The system is event-driven with message passing between tasks.

## Signal Path

```
         Preamp/Attenuator
              |
Antenna → BPF → Mixer → Crystal Filter → IF Amp → Detector → Audio Codec
           |    FST3125     10 MHz          |         |            |
        Relays  H-mode    Relay-       MCP4725    Si5351/CLK1   PCM3060
       via I2C             switched     AGC gain   BFO + AD8367  SAI1 I2S
                 Si5351    bandwidth               VGA gain         |
                 CLK0/VFO                                    Audio Mixer (DSP)
                                                              /          \
                                                      Headphones       Speaker
                                                      WM8940/SAI2    MAX98357A/I2S

TX: Audio Codec → Modulator → Mixer → LPF → HF PA (50W) → Antenna
```

## Module Boards

11 separate PCBs connected through a passive distribution board:

- **BPF** — Band-pass input filters, relay-switched per band
- **Mixer** — FST3125 H-mode mixer, Si5351 CLK0 local oscillator (25 MHz crystal, PLLA)
- **Crystal Filter** — 10 MHz IF, switchable bandwidths, noise blanker with adjustable threshold
- **IF Amplifier** — Variable gain via DAC, RSSI measurement via 12-bit ADC, preamp/attenuator
- **Detector** — Product detector with Si5351 CLK1 BFO (PLLB), AD8367 VGA for TX gain control, PCA9534 RX/TX switching
- **Audio Panel** — PCM3060 stereo codec, I2S interface to main controller
- **LPF** — Low-pass output filters with AD8307 log-detector power measurement
- **HF Power Amplifier** — 50W, driver + final stage, DAC-controlled bias, NTC temperature monitoring
- **Control Board** — Power management (3x INA228), fan, USB (CDC + UAC1), USB-C PD, CW paddle, PTT
- **Front Panel** — 3x color IPS TFT (ST7789, 240x135), encoders, buttons, LEDs, WM8940 headphone codec
- **Distribution Board** — Passive connectors routing signals between modules

## Front Panel

Three small color IPS displays (ST7789, 240x135 each) show: main tuning screen with frequency/mode/levels, real-time spectrum with waterfall, and S-meter/power/SWR readings. Rotary encoders handle tuning and parameter adjustment. The front panel controller renders all UI locally — it receives state updates over the SPI link and handles display and input logic independently.

Headphone audio goes through a WM8940 mono codec on the front panel, receiving its stream from the main controller over SAI2.

## Power System

Three INA228 high-side monitors track VBUS input, PA supply, and 3.3V rail in real time. Overcurrent protection with automatic shutdown. The 50V PA supply is enabled only when transmitting, with mode-based sequencing. PWM-controlled fan responds to PA temperature.

## Features

**Modes**: SSB (LSB/USB), CW, AM

**Receiver**:
- FST3125 H-mode mixer with Si5351 PLL synthesizer
- 10 MHz crystal filter with switchable bandwidth
- Software-controlled AGC with adjustable attack/decay
- Noise blanker with adjustable threshold
- Preamp / attenuator
- S-meter with peak hold
- Real-time spectrum display and scrolling waterfall

**Transmitter**:
- 50W output
- VOX with adjustable delay
- SWR and forward power metering (AD8307 log detectors)
- Per-rail overcurrent protection with automatic shutdown
- Thermal monitoring and active cooling

**CW**:
- Built-in iambic keyer with adjustable speed
- Sidetone generator
- Paddle input (dit/dah)

**Digital integration**:
- USB audio (UAC1) — use as a soundcard for digital modes (FT8, WSPR, etc.)
- USB serial (CDC) — CAT control for logging/rig control software + CLI
- USB-C Power Delivery — single cable for power, no wall wart

**Form factor**: Single-DIN (180x50 mm) — fits a standard car stereo slot or a compact shack setup

## Tech Stack

- **Rust** (no_std, async, embedded)
- **Embassy** — async executor, HAL, timers, sync primitives
- **2x STM32H563VI** — Cortex-M33, 250 MHz
- **Hardware DSP** — CORDIC coprocessor, FMAC FIR accelerator
- **probe-rs** for debug and flash

## Building

```bash
cargo build --release
```

## License

TBD
