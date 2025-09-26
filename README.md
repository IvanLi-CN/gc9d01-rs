# GC9D01 Rust Driver

[![Crates.io](https://img.shields.io/crates/v/gc9d01.svg)](https://crates.io/crates/gc9d01)
[![Documentation](https://docs.rs/gc9d01/badge.svg)](https://docs.rs/gc9d01)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/IvanLi-CN/gc9d01-rs)

A `no_std` Rust driver for the **GC9D01** LCD display controller with full **embedded-graphics** support. This driver provides both async and sync APIs, making it suitable for modern embedded frameworks like Embassy as well as traditional blocking applications.

## ✨ Features

- 🎯 **Complete GC9D01 support** - Full manufacturer initialization sequence from official documentation
- ⚡ **Async/Sync APIs** - Built for Embassy and other async frameworks, with sync fallback
- 🎨 **embedded-graphics integration** - Full DrawTarget implementation for rich graphics
- 🚀 **High-performance rendering** - Optimized batch SPI transfers and frame buffer architecture
- 🌈 **RGB565 color format** - 16-bit color depth with efficient memory usage
- 🔧 **Hardware abstraction** - Works with any embedded-hal SPI implementation
- 📐 **Multiple orientations** - Portrait, Landscape, and rotated variants with coordinate transformation
- 🎪 **Display optimized** - Designed for GC9D01-based LCD panels
- 📦 **no_std compatible** - Perfect for resource-constrained embedded systems

### Panel profiles

- `panel_160x50` — Enables an alternative initialization sequence and addressing model tailored for narrow modules with a 160×50 visible region (physical GRAM window 50×160 with a column offset). When this feature is on:
  - MADCTL = 0x00; column window (2A) = 0x000F..0x0040, row window (2B) = 0x0000..0x009F
  - Use `Config { width: 160, height: 50, orientation: Landscape, dx: 15, dy: 0, .. }`
  - Init sequence (GIP/porch/frame-rate/VREG/Gamma/SOU) matches the vendor direct‑SPI script used in our ESP32‑S3 example

## 🚀 Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
gc9d01 = "0.1"
embedded-graphics = "0.8"

# For async support
gc9d01 = { version = "0.1", features = ["async"] }
```

### Basic Usage

```rust
use gc9d01::{GC9D01, Config, Orientation};
use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, PrimitiveStyle},
};

// Create display configuration
// Default profile (160×40 logical)
let config = Config { width: 160, height: 40, orientation: Orientation::Portrait, rgb: false, inverted: false, dx: 0, dy: 0 };

// Initialize display (async example)
let mut display = GC9D01::new(
    config, spi_device, dc_pin, rst_pin,
    buffer, frame_buffer
);
display.init().await?;

// Draw with embedded-graphics
Circle::new(Point::new(80, 20), 30)
    .into_styled(PrimitiveStyle::with_fill(Rgb565::RED))
    .draw(&mut display)?;

// Flush to screen
display.flush().await?;
```

## 📋 Examples

This repository includes comprehensive examples for different use cases:

### 🟦 ESP32‑S3 Examples

These target a 160×50 visible region module; the init is identical between direct‑SPI and embedded‑graphics examples.

#### Direct SPI

- Location: `examples/esp32s3-160-50-direct-spi/`
- Pinout: SCK=GPIO12, MOSI=GPIO11, CS=GPIO13, DC=GPIO10, RST=GPIO14, BLK=GPIO15
- Runner: `espflash flash --monitor`

#### embedded‑graphics

- Location: `examples/esp32s3-160-50-embedded-graphics/`
- Feature: `panel_160x50` (enabled in the example)
- Contains rich demos (fills, checkerboards, shapes, lines, text, grid) using square cells

> Note: both examples include `esp_bootloader_esp_idf::esp_app_desc!()` so they can be flashed via `espflash`.

### 🎮 STM32G4 Examples

#### Direct SPI Implementation

- **Location**: `examples/stm32g4-160-40-direct-spi-90-complex-patterns/`
- **Features**: Raw SPI operations, complex pattern rendering, coordinate transformation
- **Performance**: Optimized for maximum speed with chunked rendering

#### embedded-graphics Integration

- **Location**: `examples/stm32g4-160-40-embedded-graphics/`
- **Features**: Full embedded-graphics support, shapes, text, patterns
- **Use case**: Rich graphics applications with high-level drawing APIs

#### Basic Display Test

- **Location**: `examples/stm32g4-160-40/`
- **Features**: Simple color cycling, basic functionality verification
- **Use case**: Hardware testing and driver validation

### 🔧 Hardware Configuration (STM32G4)

All examples use this pin configuration:

| Function | Pin | Description |
|----------|-----|-------------|
| SCK      | PB3 | SPI Clock |
| MOSI     | PA7 | SPI Data |
| CS       | PA4 | Chip Select (active low) |
| DC       | PB0 | Data/Command select |
| RST      | PC4 | Reset (active low) |

### 🏃‍♂️ Running Examples

```bash
# Basic functionality test
cd examples/stm32g4-160-40
cargo run

# embedded-graphics demo
cd examples/stm32g4-160-40-embedded-graphics
cargo run

# High-performance patterns
cd examples/stm32g4-160-40-direct-spi-90-complex-patterns
cargo run
```

## 🎨 embedded-graphics Support

This driver provides full `DrawTarget` implementation, supporting:

- **Primitives**: Rectangles, circles, triangles, lines
- **Text rendering**: Multiple fonts and styles
- **Images**: Bitmap and raw image support
- **Custom graphics**: Any embedded-graphics compatible drawing
- **Coordinate transformation**: Automatic handling of display orientation

```rust
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Rectangle, PrimitiveStyle},
    text::Text,
};

// Draw a rectangle
Rectangle::new(Point::new(10, 10), Size::new(50, 30))
    .into_styled(PrimitiveStyle::with_fill(Rgb565::BLUE))
    .draw(&mut display)?;

// Draw text
let text_style = MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE);
Text::new("Hello GC9D01!", Point::new(20, 25), text_style)
    .draw(&mut display)?;
```

## ⚙️ Configuration

### Display Orientations

The driver supports multiple orientations with automatic coordinate transformation:

```rust
use gc9d01::Orientation;

let config = Config {
    orientation: Orientation::Portrait,     // 0°
    // orientation: Orientation::Landscape,    // 90°
    // orientation: Orientation::PortraitSwapped,  // 180°
    // orientation: Orientation::LandscapeSwapped, // 270°
    ..Default::default()
};
```

### Performance Optimization

For maximum performance, use the frame buffer architecture:

```rust
// Example: allocate a frame buffer for 160×40 (default profile)
static mut FRAME_BUFFER: [Rgb565; 160 * 40] = [Rgb565::BLACK; 160 * 40];
let fb: &mut [Rgb565] = unsafe { &mut FRAME_BUFFER };
let mut display = GC9D01::new(config, spi_device, dc_pin, rst_pin, fb);
```

## 🔧 Features

### Cargo Features

- `async` - Enable async/await support (requires `embedded-hal-async`)
- `defmt` - Enable defmt logging support
 - `panel_160x50` - Initialization + addressing for 160×50 visible region modules

```toml
[dependencies]
gc9d01 = { version = "0.1", features = ["async", "defmt", "panel_160x50"] }

// Typical config for the panel_160x50 profile:
// Config { width: 160, height: 50, orientation: Landscape, dx: 15, dy: 0, .. }

// For direct parity with the vendor script, ensure MADCTL=0x00 is applied by init (handled by the feature).

### Breaking change notice

- Default `Config` now targets 160×40 logical rendering.
- Legacy constants like `FRAME_BUF_SIZE`/`MAX_FRAME_PIXELS` have been aligned accordingly. Prefer using `Config::frame_bytes()` and `Config::frame_pixels()` to avoid breakage when switching profiles.
```

## 🦀 Rust Version Requirements

This crate requires **Rust 1.85** or later and uses the **2024 edition**. This enables:

- Enhanced async/await support with improved Future handling
- Better error diagnostics and compiler messages
- Optimized code generation for embedded targets

## 📚 Documentation

- 📖 [API Documentation](https://docs.rs/gc9d01)
- 📄 [GC9D01 Datasheet](docs/GC9D01N%20DataSheet%20V1.1.pdf)
- 🔧 [Manufacturer Initialization Sequence](docs/GC9D01+HSD0.99(HSD010BPW3)IPS_20210914_ruien_V2(1).txt)

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

## 📄 License

This project is dual-licensed under either:

- MIT License ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)

at your option.

## 🙏 Acknowledgments

- Based on the official GC9D01 initialization sequence
- Inspired by the embedded-hal ecosystem
- Built for the Embassy async framework
