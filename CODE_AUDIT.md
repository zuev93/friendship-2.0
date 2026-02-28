# Code Audit - Friendship 2.0

Date: 2026-02-28

Hacks, workarounds, and insufficiencies found across the codebase.

---

## Critical (Rule Violations)

### 2. `#[allow(dead_code)]` attributes (violates rule #4)

**Partially fixed:**
- `app/types.rs` — Removed from ClarifierMode, RfGainMode (all variants used). FilterType kept (firmware incomplete).
- `drivers/wm8940.rs` — Register enum trimmed to used variants only, `#[allow(dead_code)]` removed.

**Remaining:**
- `display/modules/display.rs:10,17,26` — Color enum, display methods
- `display/modules/framebuffer.rs:16,25,146` — FrameBuffer methods
- `display/types.rs:18` — display types
- `peripherals/modules/hf_amp.rs:14` — gpio field never used
- `peripherals/types.rs:1` — peripheral types
- `drivers/ad9834.rs:29,31` — Triangle, Square waveform variants
- `drivers/ads1115.rs:17,30,43` — Gain, DataRate, ComparatorQueue enums
- `drivers/sc18is602.rs:44,46,48,55,57,59,66` — unused SPI modes, rates, LsbFirst
- `drivers/ssd1315.rs:11` — DISPLAY_WIDTH constant

---

### End-to-End Status

**Signal sent but no consumer** (encoder mapped, event dispatched, but nothing acts on it):
- **Menu** (ID 8): `MENU_ENCODER_EVENTS` channel exists, no consumer task

## Low Priority (Stubs / Incomplete)

### 14. 26 TODO comments in main-controller — unchanged

### 15. Error channel has no consumer — unchanged

### 16. Display subsystem not fully integrated — unchanged
