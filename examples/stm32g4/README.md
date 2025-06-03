# GC9D01 Display Example for STM32G4

This example demonstrates how to configure and interact with the GC9D01 display controller using an STM32G4 microcontroller and the `gc9d01-rs` driver.

## Configuration

The following parameters are relevant for this example:

* **Display Resolution:** 40x160 pixels
* **Interface:** SPI
* **SPI Pins:**
  * SCK: (To be configured in `main.rs`)
  * MOSI: (To be configured in `main.rs`)
  * MISO: (Optional, if used by your SPI peripheral and driver, to be configured in `main.rs`)
* **Control Pins:**
  * CS (Chip Select): (To be configured in `main.rs`)
  * DC (Data/Command): (To be configured in `main.rs`)
  * RST (Reset): (Optional, but recommended, to be configured in `main.rs`)
  * BLK (Backlight): (Optional, to be configured in `main.rs`)

## Usage

To run this example, ensure you have the necessary Rust toolchain and `embassy-stm32` dependencies installed.

1. **Build the project:**

    ```bash
    cargo build --target thumbv7em-none-eabihf # Adjust target if necessary
    ```

    (Note: The example name for `cargo build --example <name>` is derived from the `[[bin]] name` in `Cargo.toml`, which is `gc9d01_stm32g4_example`. If this example is part of a workspace and defined in the root `Cargo.toml`'s `[workspace.members]` or `[workspace.default-members]`, you might use `cargo build --example gc9d01_stm32g4_example`. For a standalone example not in a workspace manifest, building the binary directly is common.)

2. **Flash to your STM32G4 board:**
    (Instructions for flashing will depend on your specific setup, e.g., using `probe-rs` or STM32CubeProgrammer)

    ```bash
    # Example with probe-rs (adjust chip as needed, e.g., STM32G431CBUx)
    probe-rs run --chip STM32G431CBUx target/thumbv7em-none-eabihf/debug/gc9d01_stm32g4_example
    # or using cargo flash if probe-rs is configured as a runner
    # cargo flash --chip STM32G431CBUx --target thumbv7em-none-eabihf --bin gc9d01_stm32g4_example
    ```

3. **Monitor serial output:**
    Use a serial terminal (e.g., `minicom`, `screen`, or VS Code's serial monitor) to view the `defmt` logs if enabled and configured.

4. **Alternative: Reset and Attach to MCU with probe-rs:**

    ```bash
    probe-rs reset --chip STM32G431CBUx
    probe-rs attach --chip STM32G431CBUx target/thumbv7em-none-eabihf/debug/gc9d01_stm32g4_example
    ```

## Important Notes

* Ensure your hardware connections for SPI (SCK, MOSI, CS, DC) and any other control pins (RST, BLK) for the GC9D01 are correct.
* Pin configurations for SPI and control lines need to be set up correctly in `src/main.rs` according to your specific STM32G4 board and connections.
* This example targets a 40x160 resolution display.
