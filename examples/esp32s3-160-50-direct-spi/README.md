# ESP32‑S3 Direct SPI example (GC9D01, 160×50)

This example drives a GC9D01 display on an ESP32‑S3 “directly” over SPI without higher‑level wrappers, making it convenient to verify low‑level commands and waveforms. It mirrors the initialization used by the STM32 reference examples, and uses the same pinout as the `iso-usb-hub_v2` project.

- Location: `examples/esp32s3-160-50-direct-spi/`
- Logical resolution used in this demo area: 160 × 50 (the panel’s GRAM is 360×360; we render into a 160×50 window)
- SPI: SPI2, Mode 0, 10 MHz default

## Hardware wiring

| Display pin | Function         | ESP32‑S3 pin |
|-------------|------------------|--------------|
| SCLK/CLK    | SPI clock        | `GPIO12`     |
| MOSI/SDA    | SPI MOSI         | `GPIO11`     |
| CS          | Chip select (L)  | `GPIO13`     |
| DC          | Data/Command     | `GPIO10`     |
| RST         | Reset (active L) | `GPIO14`     |
| BLK/LED     | Backlight (H on) | `GPIO15`     |
| VCC         | Power            | 3.3 V        |
| GND         | Ground           | GND          |

Notes:

- Backlight typically at 3.0–3.3 V; ensure current is within capability, add a series resistor or external driver if needed.
- Confirm logic levels for CS/DC/RST and SPI mode 0.

## Tooling

This directory ships with `rust-toolchain.toml` using channel `esp`, target `xtensa-esp32s3-none-elf`, and `rust-src` component. `espflash` is required for flashing:

```bash
cargo install espflash
```

## Build & flash

The `.cargo/config.toml` sets:

- target: `xtensa-esp32s3-none-elf`
- runner: `espflash flash --monitor`
- `DEFMT_LOG=info`

Quick start:

```bash
cd examples/esp32s3-160-50-direct-spi
cargo run --release
```

Build only:

```bash
cargo build --release
```

## What you’ll see

- Serial log messages including:
  - `Starting GC9D01 Direct SPI Test Firmware (ESP32-S3)`
  - `init.time: embassy-timer=ok`
  - init logs followed by `Rendering tests completed. Example will idle.`
- Display output:
  - Full GRAM (360×360) is cleared to black first
  - Then, within a 160×40 region, the top row shows eight 20×20 colored squares (magenta/red/yellow/green/cyan/blue/purple/white) and the bottom row shows eight 20×20 grayscale squares (black→white)
  - For a richer embedded‑graphics demo (shapes/text/checkerboards/grid), see `examples/esp32s3-160-50-embedded-graphics` (init matches this example).

## Code map & tunables

- Main: `examples/esp32s3-160-50-direct-spi/src/main.rs`
  - SPI configuration near `Spi::new(...).with_sck(GPIO12).with_mosi(GPIO11)` (10 MHz, Mode 0 by default)
  - Pins: `CS=GPIO13`, `DC=GPIO10`, `RST=GPIO14`, `BLK=GPIO15`
  - Init sequence: identical to the `panel_160x50` profile (MADCTL=0x00, 2A/2B with column offset), comparable to the STM32 direct‑SPI example for register differences
  - Batch fill: `fill_area_with_color` uses 512‑byte bursts (256 pixels per batch)
  - Logical region: 160×40; adjust block sizes or coordinates if you want a different layout

> Troubleshooting:
>
> - Lower SPI frequency (e.g., 5 MHz) if you see flicker or noise
> - Double‑check CS/DC/RST and backlight levels
> - Some boards label the pins SDA/SCL but are wired for 3/4‑wire SPI

## License & credits

- License: MIT/Apache‑2.0 (same as repository)
- Credits:
  - Init & direct‑SPI approach adapted from `examples/stm32g4-160-40-direct-spi`
  - Pin assignment mirrors `iso-usb-hub_v2`
