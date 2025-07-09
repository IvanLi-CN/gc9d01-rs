# GC9D01 Direct SPI Example for STM32G4

This example demonstrates a complete, optimized implementation for driving the GC9D01 240x240 circular LCD display using direct SPI communication with an STM32G4 microcontroller. This implementation includes the full manufacturer initialization sequence for optimal display performance.

## Features

* **Complete GC9D01 initialization** - Full manufacturer-specified initialization sequence
* **High-performance batch rendering** - Optimized SPI communication with 256-pixel batches
* **RGB565 color support** - 16-bit color depth
* **Hardware reset sequence** - Proper display reset timing
* **Comprehensive debugging** - Detailed defmt logging for troubleshooting

## Hardware Configuration

* **Display:** GC9D01 240x240 circular LCD (RGB565)
* **Interface:** SPI (TX-only, 16MHz)
* **Pin Configuration:**
  * **SCK:** PB3 (SPI1 Clock)
  * **MOSI:** PA7 (SPI1 Master Out)
  * **CS:** PA4 (Chip Select, active low)
  * **DC:** PB0 (Data/Command select)
  * **RST:** PC4 (Reset, active low)

## What This Example Does

The example performs a continuous color cycling test:

1. **Initialization Phase:**
   * Hardware reset sequence (10ms reset pulse + 120ms stabilization)
   * Complete GC9D01 register configuration (100+ commands)
   * Internal voltage settings, GIP timing, and gamma correction
   * Display activation and memory write preparation

2. **Display Test Loop:**
   * Fills entire 240x240 screen with solid colors in sequence:
     * Red (RGB565: 0xF800)
     * Green (RGB565: 0x07E0)
     * Blue (RGB565: 0x001F)
     * Black (RGB565: 0x0000)
   * Each color displays for 3 seconds (1 second for black)
   * Uses optimized batch rendering (256 pixels per SPI transaction)

## Usage

### Prerequisites

* Rust toolchain with `thumbv7em-none-eabihf` target
* Embassy async framework
* STM32G4 development board
* GC9D01 240x240 circular LCD display
* Proper wiring connections

### Building and Running

1. **Clean and build:**

    ```bash
    cargo clean
    cargo run
    ```

2. **The build system will:**
   * Compile the project for STM32G431CBUx
   * Automatically detect and use available probe (ST-Link or ESP-JTAG)
   * Flash the firmware and start execution

3. **Expected behavior:**
   * Fast, smooth color transitions across the entire display
   * Detailed initialization logs via defmt
   * Continuous color cycling with timing information

## Performance Features

* **Optimized SPI Communication:** Batch transfers of 256 pixels (512 bytes) per transaction
* **Minimal CS Switching:** Reduces SPI overhead by ~256x compared to pixel-by-pixel transfer
* **Complete Initialization:** Full manufacturer-specified register configuration
* **Hardware Reset:** Proper timing for reliable display startup
* **Debug Logging:** Comprehensive defmt output for troubleshooting

## Technical Details

* **SPI Frequency:** 16MHz
* **Color Format:** RGB565 (16-bit)
* **Display Size:** 240x240 pixels (57,600 total pixels)
* **Refresh Performance:** Full screen fill in milliseconds (vs. seconds with naive implementation)
* **Memory Usage:** 512-byte batch buffer for optimal performance

## Implementation Highlights

This example showcases several important techniques for embedded display drivers:

1. **Complete Manufacturer Initialization:** Based on the official GC9D01 initialization sequence from the manufacturer documentation
2. **Batch SPI Transfers:** Dramatically improves performance by reducing transaction overhead
3. **Proper Hardware Reset:** Ensures reliable display startup
4. **Async/Await Pattern:** Uses Embassy's async framework for efficient resource utilization
5. **Comprehensive Logging:** Detailed debug output for development and troubleshooting
