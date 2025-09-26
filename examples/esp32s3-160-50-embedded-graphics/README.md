# ESP32‑S3 + GC9D01 (160×50) — embedded‑graphics demo

This example demonstrates how to drive a GC9D01 display module with a 160×50 visible area on an ESP32‑S3 using the `gc9d01` crate and the embedded‑graphics ecosystem. It enables the `panel_160x50` profile so that the init sequence and addressing match the vendor direct‑SPI script exactly.

- Location: `examples/esp32s3-160-50-embedded-graphics/`
- Logical size: 160 × 50 (Landscape)
- Physical addressing: 50 × 160 (columns × rows, with column offset)
- Offsets: `dx=15`, `dy=0` (matches direct‑SPI example and vendor script)
- SPI: SPI2, Mode 0, 10 MHz by default
- Init: identical to `examples/esp32s3-160-50-direct-spi` (MADCTL=0x00, 2A/2B with offset)

## Hardware wiring (same as direct‑SPI example)

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

> Note: Ensure the backlight current is within the board’s capability; use a current‑limit resistor or external driver if needed.

## Toolchain & dependencies

This directory contains `rust-toolchain.toml` and `.cargo/config.toml`:

- Toolchain channel: `esp`
- Target: `xtensa-esp32s3-none-elf`
- Runner: `espflash flash --monitor --log-format defmt`
- Includes `esp_bootloader_esp_idf::esp_app_desc!()` so `espflash` can flash the image

Install `espflash` if you haven’t:

```bash
cargo install espflash
```

## Build & flash

```bash
cd examples/esp32s3-160-50-embedded-graphics
cargo run --release
```

Build only:

```bash
cargo build --release
```

## Driver/profile notes

- Enable `gc9d01` feature `panel_160x50` (already enabled here). Key init parameters:
  - `MADCTL=0x00`, `EC=0x00`, `SOU=0x40`
  - 2A (column): `0x000F..0x0040` (width 50, includes offset 15)
  - 2B (row): `0x0000..0x009F` (height 160)
  - GIP/porch/frame‑rate/VREG/Gamma exactly match the vendor script
- `flush()` and all address windows honor `dx/dy` so the active area aligns (no right‑edge blanking).
- Coordinates: The example renders a 160×50 logical scene in Landscape while the driver maps to the 50×160 physical addressing.

## What you’ll see

The demo sequence includes fully covered square‑cell patterns (clipped at edges when needed):

1. Solid fills: red/green/blue
2. 20×20 multi‑color checkerboard (2 rows × 8 columns)
3. 10×10 checkerboard (16 × 5, full screen)
4. Basic shapes: stroked/filled rectangles and circles
5. Lines: 8 diagonal sets with RGB cycling
6. Text: two lines using 6×10 font
7. Square grid (10×10): black/white cells

## FAQ

- Shifted image / right‑edge blank: ensure `panel_160x50` is enabled and `dx=15` is set.
- Flicker/noise: try a lower SPI clock (e.g., 5 MHz) and check wiring/grounding/power.
- Orientation: for Portrait, change `orientation` accordingly and adjust coordinates if needed.

## Related examples

- Direct SPI (same init, low‑level command test): `examples/esp32s3-160-50-direct-spi`
- STM32G4 + embedded‑graphics (160×40 reference): `examples/stm32g4-160-40-embedded-graphics`

## License

Same as the repository (MIT/Apache‑2.0).
