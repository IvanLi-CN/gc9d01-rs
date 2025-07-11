# STM32G4 90° Complex Patterns Example

This example demonstrates complex pattern rendering on a GC9D01 display using the gc9d01 library in 90° orientation (Landscape).

## Features

- **Library-based implementation**: Uses the gc9d01 Rust library instead of direct SPI operations
- **90° orientation**: Demonstrates Portrait orientation with 160×40 logical configuration
- **Complex patterns**: Shows multiple sophisticated rendering patterns
- **Async/await**: Built with Embassy async framework
- **High-level API**: Uses the library's `write_area()` and `fill_color()` methods

## Hardware Requirements

- STM32G431CB microcontroller
- GC9D01 circular LCD display (configured as 160×40 with Portrait orientation)
- SPI connection with the following pin mapping:
  - SCK: PB3
  - MOSI: PA7
  - CS: PA4
  - DC: PB0
  - RST: PC4

## Patterns Demonstrated

### 1. Complex Multi-Color Checkerboard
- 8×2 grid of 20×20 pixel blocks
- Uses 10 different colors in sequence
- Demonstrates precise area rendering

### 2. Gradient Color Stripes
- 10 vertical stripes with gradient effects
- Red, green, and blue gradients alternating
- Shows smooth color transitions

### 3. Concentric Rectangles
- 5 layers of nested rectangles
- Different colors for each layer
- Demonstrates border drawing techniques

### 4. Diagonal Lines Pattern
- 20 diagonal lines across the screen
- Multiple colors with spacing
- Shows geometric pattern rendering

### 5. Spiral Pattern
- Mathematical spiral from center outward
- Two full rotations with color changes
- Demonstrates coordinate calculations

## Building and Running

```bash
cd examples/stm32g4-90-complex-patterns
cargo run
```

## Key Differences from Direct SPI Examples

1. **High-level API**: Uses `display.write_area()` instead of manual SPI commands
2. **Automatic initialization**: Library handles the complete GC9D01 initialization sequence
3. **Coordinate transformation**: Library automatically handles Portrait orientation coordinate mapping
4. **Memory management**: Uses library's internal buffer management
5. **Error handling**: Proper Result types for all operations

## Code Structure

- **Timer implementation**: Custom `EmbassyDisplayTimer` implementing the library's Timer trait
- **SPI setup**: Uses Embassy's shared SPI bus with proper device abstraction
- **Display configuration**: Configures 160×40 resolution with Portrait orientation (matches stm32g4 example)
- **Pattern loop**: Continuous demonstration of all patterns with 10-second intervals

## Technical Notes

- Uses `embedded-graphics` Rgb565 color format
- Implements proper async/await patterns throughout
- Handles memory allocation constraints in no_std environment
- Demonstrates efficient pixel data management for complex patterns
