# STM32G4 Embedded Graphics Example

This example demonstrates how to use the GC9D01 driver with the embedded-graphics library on an STM32G4 microcontroller.

## Features

- **Embedded Graphics Integration**: Full support for embedded-graphics drawing primitives
- **Frame Buffer Architecture**: All rendering operations use memory buffers for smooth graphics
- **90° Orientation**: Configured for 160×40 logical display in landscape mode
- **Async/await**: Built with Embassy async framework
- **Complex Graphics**: Demonstrates circles, rectangles, lines, text, and custom patterns

## Hardware Requirements

- STM32G4 series microcontroller (tested on STM32G431CBUx)
- GC9D01 display (40×160 physical pixels, used in 90° rotation)
- SPI connections:
  - SCK: PB3
  - MOSI: PA7
  - CS: PA4
  - DC: PB0
  - RST: PC4

## Display Configuration

- **Physical Display**: 40×160 pixels
- **Logical Configuration**: 160×40 pixels (90° rotated)
- **Color Format**: RGB565 (16-bit)
- **Frame Buffer**: Full-screen memory buffer for smooth rendering

## Building and Running

```bash
cd examples/stm32g4-embedded-graphics
cargo clean
cargo run
```

## Graphics Demonstrations

The example showcases various embedded-graphics features:

1. **Basic Shapes**: Circles, rectangles, triangles
2. **Lines and Patterns**: Various line styles and geometric patterns
3. **Text Rendering**: Different fonts and text positioning
4. **Color Gradients**: Smooth color transitions
5. **Animation**: Moving graphics with frame buffer updates

## Key Differences from Direct SPI Examples

1. **High-level Graphics API**: Uses embedded-graphics primitives instead of manual pixel operations
2. **Automatic Coordinate Handling**: embedded-graphics handles coordinate transformations
3. **Rich Drawing Primitives**: Access to circles, text, complex shapes out of the box
4. **Composable Graphics**: Easy to combine multiple drawing operations
5. **Standard Graphics Interface**: Compatible with the broader embedded-graphics ecosystem

## Code Structure

- `main.rs`: Main application with embedded-graphics demonstrations
- Hardware initialization follows the same pattern as other STM32G4 examples
- Display driver configured with frame buffer support for embedded-graphics compatibility
