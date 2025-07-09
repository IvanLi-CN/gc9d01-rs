# GC9D01 Rust Driver

A Rust driver for the GC9D01 240x240 circular LCD display controller, based on [embedded-hal](https://crates.io/crates/embedded-hal).

## Features

* **Complete GC9D01 support** - Full manufacturer initialization sequence
* **Async/await support** - Built for Embassy and other async frameworks
* **High-performance rendering** - Optimized batch SPI transfers
* **RGB565 color format** - 16-bit color depth
* **Hardware abstraction** - Works with any embedded-hal SPI implementation

## Examples

### STM32G4 Direct SPI Example

Located in `examples/stm32g4-direct-spi/`, this example demonstrates:

* **Complete GC9D01 initialization** - Full manufacturer-specified register configuration
* **High-performance batch rendering** - 256-pixel batch transfers for optimal speed
* **Hardware reset sequence** - Proper timing for reliable display startup
* **Color cycling test** - Continuous RGB color demonstration
* **Embassy async framework** - Modern async/await patterns

**Hardware Configuration:**

* Display: GC9D01 240x240 circular LCD
* Interface: SPI (16MHz, TX-only)
* Pins: SCK(PB3), MOSI(PA7), CS(PA4), DC(PB0), RST(PC4)

**Usage:**

```bash
cd examples/stm32g4-direct-spi
cargo clean
cargo run
```

**Performance:** Full screen refresh in milliseconds (vs. seconds with naive implementation)

## Documentation

* [GC9D01 Datasheet](docs/GC9D01N%20DataSheet%20V1.1.pdf)
* [Manufacturer Initialization Sequence](docs/GC9D01+HSD0.99(HSD010BPW3)IPS_20210914_ruien_V2(1).txt)

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
