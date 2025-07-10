#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::spi::{Config as SpiConfig, Spi};
use embassy_stm32::mode::Async;
use embassy_stm32::time::Hertz;
use embassy_time::Timer;

use {defmt_rtt as _, panic_probe as _};
use defmt::*;

// GC9D01 initialization function following the official reference document
async fn initialize_gc9d01(
    spi: &mut Spi<'_, Async>,
    cs_pin: &mut Output<'_>,
    dc_pin: &mut Output<'_>,
    rst_pin: &mut Output<'_>,
) {
    // Helper function to send command with multiple data bytes
    async fn write_command_with_data(spi: &mut Spi<'_, Async>, cs: &mut Output<'_>, dc: &mut Output<'_>, cmd: u8, data: &[u8]) {
        // Send command
        dc.set_low();  // Command mode
        cs.set_low();  // Select device
        let _ = spi.write(&[cmd]).await;
        cs.set_high(); // Deselect device

        // Send data if any
        if !data.is_empty() {
            dc.set_high(); // Data mode
            cs.set_low();  // Select device
            let _ = spi.write(data).await;
            cs.set_high(); // Deselect device
        }
    }

    async fn write_command(spi: &mut Spi<'_, Async>, cs: &mut Output<'_>, dc: &mut Output<'_>, cmd: u8) {
        dc.set_low();  // Command mode
        cs.set_low();  // Select device
        let _ = spi.write(&[cmd]).await;
        cs.set_high(); // Deselect device
    }

    // GC9D01 hardware reset sequence
    info!("Performing GC9D01 hardware reset...");
    rst_pin.set_low();   // Assert reset (active low)
    Timer::after_millis(10).await;  // Hold reset for 10ms
    rst_pin.set_high();  // Release reset
    Timer::after_millis(120).await; // Wait for display to stabilize (120ms)
    info!("GC9D01 hardware reset completed");

    // GC9D01 complete initialization sequence based on reference document
    info!("Starting GC9D01 initialization sequence from reference document...");

    // Enable internal register access (0xFE, 0xEF)
    write_command(spi, cs_pin, dc_pin, 0xFE).await;
    write_command(spi, cs_pin, dc_pin, 0xEF).await;
    info!("Internal register access enabled");

    // Internal register settings 0x80-0x8F (all set to 0xFF) - exactly as in reference
    info!("Setting internal registers 0x80-0x8F...");
    for reg in 0x80..=0x8F {
        write_command_with_data(spi, cs_pin, dc_pin, reg, &[0xFF]).await;
    }

    // Pixel format setting - RGB565 (16-bit color)
    write_command_with_data(spi, cs_pin, dc_pin, 0x3A, &[0x05]).await;
    info!("Pixel format set to RGB565");

    // Display rotation setting
    write_command_with_data(spi, cs_pin, dc_pin, 0xEC, &[0x11]).await;
    info!("Display rotation configured");

    // VGL size setting
    write_command_with_data(spi, cs_pin, dc_pin, 0x7E, &[0x7A]).await;
    info!("VGL size configured");

    // Frame frequency modification
    write_command_with_data(spi, cs_pin, dc_pin, 0x74, &[
        0x02, 0x0E, 0x00, 0x00, 0x28, 0x00, 0x00
    ]).await;
    info!("Frame frequency configured");

    // Internal voltage adjustment
    write_command_with_data(spi, cs_pin, dc_pin, 0x98, &[0x3E]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0x99, &[0x3E]).await;
    info!("Internal voltage adjusted");

    // Internal porch settings
    write_command_with_data(spi, cs_pin, dc_pin, 0xB5, &[0x0E, 0x0E]).await;
    info!("Internal porch configured");

    // GIP timing start - exactly as in reference document
    info!("Configuring GIP timing...");
    write_command_with_data(spi, cs_pin, dc_pin, 0x60, &[
        0x38, 0x09, 0x6D, 0x67
    ]).await;

    write_command_with_data(spi, cs_pin, dc_pin, 0x63, &[
        0x38, 0xAD, 0x6D, 0x67, 0x05
    ]).await;

    write_command_with_data(spi, cs_pin, dc_pin, 0x64, &[
        0x38, 0x0B, 0x70, 0xAB, 0x6D, 0x67
    ]).await;

    write_command_with_data(spi, cs_pin, dc_pin, 0x66, &[
        0x38, 0x0F, 0x70, 0xAF, 0x6D, 0x67
    ]).await;

    write_command_with_data(spi, cs_pin, dc_pin, 0x6A, &[
        0x00, 0x00
    ]).await;

    write_command_with_data(spi, cs_pin, dc_pin, 0x68, &[
        0x3B, 0x08, 0x04, 0x00, 0x04, 0x64, 0x67
    ]).await;

    write_command_with_data(spi, cs_pin, dc_pin, 0x6C, &[
        0x22, 0x02, 0x22, 0x02, 0x22, 0x22, 0x50
    ]).await;

    // Long 0x6E command with all data from reference
    write_command_with_data(spi, cs_pin, dc_pin, 0x6E, &[
        0x00, 0x00, 0x00, 0x00, 0x07, 0x01, 0x13, 0x11,
        0x0B, 0x09, 0x16, 0x15, 0x1D, 0x1E, 0x00, 0x00,
        0x00, 0x00, 0x1E, 0x1D, 0x15, 0x16, 0x0A, 0x0C,
        0x12, 0x14, 0x02, 0x08, 0x00, 0x00, 0x00, 0x00
    ]).await;
    info!("GIP timing configured");

    // Internal voltage settings start - from reference document
    info!("Configuring internal voltage settings...");
    write_command_with_data(spi, cs_pin, dc_pin, 0xA9, &[0x1B]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0xA8, &[0x6B]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0xA8, &[0x6D]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0xA7, &[0x40]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0xAD, &[0x47]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0xAF, &[0x73]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0xAF, &[0x73]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0xAC, &[0x44]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0xA3, &[0x6C]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0xCB, &[0x00]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0xCD, &[0x22]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0xC2, &[0x10]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0xC5, &[0x00]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0xC6, &[0x0E]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0xC7, &[0x1F]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0xC8, &[0x0E]).await;
    info!("Internal voltage settings configured");

    // Single gate mode selection
    write_command_with_data(spi, cs_pin, dc_pin, 0xBF, &[0x00]).await;

    // SOU related adjustment
    write_command_with_data(spi, cs_pin, dc_pin, 0xF9, &[0x20]).await;

    // VREG voltage adjustment
    write_command_with_data(spi, cs_pin, dc_pin, 0x9B, &[0x3B]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0x93, &[0x33, 0x7F, 0x00]).await;

    // VGH/VGL CLK adjustment
    write_command_with_data(spi, cs_pin, dc_pin, 0x70, &[
        0x0E, 0x0F, 0x03, 0x0E, 0x0F, 0x03
    ]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0x71, &[
        0x0E, 0x16, 0x03
    ]).await;

    // Internal voltage adjustment
    write_command_with_data(spi, cs_pin, dc_pin, 0x91, &[0x0E, 0x09]).await;

    // VREG voltage adjustment
    write_command_with_data(spi, cs_pin, dc_pin, 0xC3, &[0x2C]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0xC4, &[0x1A]).await;

    // Gamma settings F0-F3
    info!("Configuring gamma settings...");
    write_command_with_data(spi, cs_pin, dc_pin, 0xF0, &[
        0x51, 0x13, 0x0C, 0x06, 0x00, 0x2F
    ]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0xF2, &[
        0x51, 0x13, 0x0C, 0x06, 0x00, 0x33
    ]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0xF1, &[
        0x3C, 0x94, 0x4F, 0x33, 0x34, 0xCF
    ]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0xF3, &[
        0x4D, 0x94, 0x4F, 0x33, 0x34, 0xCF
    ]).await;
    info!("Gamma settings configured");

    // Memory access control - exactly as in reference document
    write_command_with_data(spi, cs_pin, dc_pin, 0x36, &[0x40]).await;
    info!("Memory access control configured");

    // Sleep out command - wake up the display
    write_command(spi, cs_pin, dc_pin, 0x11).await;
    Timer::after_millis(200).await; // Wait 200ms as specified in reference
    info!("Sleep out command sent");

    // Display on command
    write_command(spi, cs_pin, dc_pin, 0x29).await;

    // Memory write command - ready for pixel data
    write_command(spi, cs_pin, dc_pin, 0x2C).await;
    Timer::after_millis(100).await;
    info!("Display on command sent");

    info!("GC9D01 initialization complete!");
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Starting GC9D01 90° Simple Checkerboard Test Firmware");

    let mut config = embassy_stm32::Config::default();
    {
        use embassy_stm32::rcc::*;
        config.rcc.hsi48 = Some(Hsi48Config {
            sync_from_usb: true,
        });
        config.rcc.pll = Some(Pll {
            source: PllSource::HSI,
            prediv: PllPreDiv::DIV4,
            mul: PllMul::MUL85,
            divp: None,
            divq: None,
            divr: Some(PllRDiv::DIV2),
        });
        config.rcc.mux.adc12sel = mux::Adcsel::SYS;
        config.rcc.sys = Sysclk::PLL1_R;
        config.rcc.mux.clk48sel = mux::Clk48sel::HSI48;
    }
    let p = embassy_stm32::init(config);

    info!("Hardware initialized successfully");

    // SPI and GPIO initialization
    info!("Initializing SPI and GPIO pins...");

    // Configure GPIO pins for GC9D01
    let sck_pin = p.PB3;   // SPI Clock
    let mosi_pin = p.PA7;  // SPI MOSI (Master Out Slave In)
    let mut cs_pin = Output::new(p.PA4, Level::High, Speed::VeryHigh);  // Chip Select (active low)
    let mut dc_pin = Output::new(p.PB0, Level::Low, Speed::VeryHigh);   // Data/Command select
    let mut rst_pin = Output::new(p.PC4, Level::Low, Speed::VeryHigh);  // Reset (active low)

    // Configure SPI1 for GC9D01 communication
    let mut spi_config = SpiConfig::default();
    spi_config.frequency = Hertz(16_000_000); // 16MHz SPI frequency

    let mut spi = Spi::new_txonly(
        p.SPI1,
        sck_pin,
        mosi_pin,
        p.DMA1_CH1,
        spi_config,
    );

    info!("SPI and GPIO initialized successfully");

    // SPI communication helper functions with debug logging
    async fn write_command(spi: &mut Spi<'_, Async>, cs: &mut Output<'_>, dc: &mut Output<'_>, cmd: u8) {
        debug!("Sending command: 0x{:02X}", cmd);
        dc.set_low();  // Command mode
        cs.set_low();  // Select device
        let _ = spi.write(&[cmd]).await;
        cs.set_high(); // Deselect device
    }

    async fn write_data_slice(spi: &mut Spi<'_, Async>, cs: &mut Output<'_>, dc: &mut Output<'_>, data: &[u8]) {
        debug!("Sending {} bytes of data", data.len());
        dc.set_high(); // Data mode
        cs.set_low();  // Select device
        let _ = spi.write(data).await;
        cs.set_high(); // Deselect device
    }

    // Helper function to send command with multiple data bytes
    async fn write_command_with_data(spi: &mut Spi<'_, Async>, cs: &mut Output<'_>, dc: &mut Output<'_>, cmd: u8, data: &[u8]) {
        write_command(spi, cs, dc, cmd).await;
        if !data.is_empty() {
            write_data_slice(spi, cs, dc, data).await;
        }
    }

    // Set display address window (rectangular area)
    async fn set_address_window(
        spi: &mut Spi<'_, Async>,
        cs: &mut Output<'_>,
        dc: &mut Output<'_>,
        x0: u16, y0: u16, x1: u16, y1: u16
    ) {
        // Column address set (0x2A)
        write_command(spi, cs, dc, 0x2A).await;
        write_data_slice(spi, cs, dc, &[
            (x0 >> 8) as u8, (x0 & 0xFF) as u8,  // Start column high, low
            (x1 >> 8) as u8, (x1 & 0xFF) as u8   // End column high, low
        ]).await;

        // Row address set (0x2B)
        write_command(spi, cs, dc, 0x2B).await;
        write_data_slice(spi, cs, dc, &[
            (y0 >> 8) as u8, (y0 & 0xFF) as u8,  // Start row high, low
            (y1 >> 8) as u8, (y1 & 0xFF) as u8   // End row high, low
        ]).await;
    }

    // Start memory write operation
    async fn start_memory_write(spi: &mut Spi<'_, Async>, cs: &mut Output<'_>, dc: &mut Output<'_>) {
        write_command(spi, cs, dc, 0x2C).await; // Memory write command
    }

    // Fill area with solid color - optimized for 90° orientation
    async fn fill_area_with_color_90(
        spi: &mut Spi<'_, Async>,
        cs: &mut Output<'_>,
        dc: &mut Output<'_>,
        x0: u16, y0: u16, x1: u16, y1: u16,
        color: u16
    ) {
        let width = x1 - x0 + 1;
        let height = y1 - y0 + 1;
        let pixel_count = (width as u32) * (height as u32);

        debug!("Filling logical area ({},{}) to ({},{}) with color 0x{:04X} in 90°+180° orientation",
               x0, y0, x1, y1, color);

        // Transform logical coordinates (160x40) to physical coordinates (40x160) for 90° + 180° rotation
        // 90° rotation: logical(x,y) -> physical(y, x)
        // Then 180° rotation: physical(a,b) -> physical(39-a, 159-b)
        // Combined: logical(x,y) -> physical(39-y, 159-x)
        let physical_x0 = 39 - y1;  // Note: y1 maps to x0 (inverted)
        let physical_y0 = 159 - x1; // Note: x1 maps to y0 (inverted)
        let physical_x1 = 39 - y0;  // Note: y0 maps to x1 (inverted)
        let physical_y1 = 159 - x0; // Note: x0 maps to y1 (inverted)

        debug!("Transformed to physical coordinates ({},{}) to ({},{})",
               physical_x0, physical_y0, physical_x1, physical_y1);

        set_address_window(spi, cs, dc, physical_x0, physical_y0, physical_x1, physical_y1).await;
        start_memory_write(spi, cs, dc).await;

        // Prepare color bytes
        let color_bytes = [(color >> 8) as u8, (color & 0xFF) as u8];

        // Use batch sending for better performance
        const BATCH_SIZE: usize = 512; // Send 256 pixels at a time (512 bytes)
        let mut batch_buffer = [0u8; BATCH_SIZE];

        // Fill batch buffer with repeated color pattern
        for i in (0..BATCH_SIZE).step_by(2) {
            batch_buffer[i] = color_bytes[0];
            batch_buffer[i + 1] = color_bytes[1];
        }

        let pixels_per_batch = BATCH_SIZE / 2;
        let full_batches = pixel_count as usize / pixels_per_batch;
        let remaining_pixels = pixel_count as usize % pixels_per_batch;

        debug!("Sending {} full batches of {} pixels each", full_batches, pixels_per_batch);

        // Send full batches
        for batch in 0..full_batches {
            dc.set_high(); // Data mode
            cs.set_low();  // Select device
            let _ = spi.write(&batch_buffer).await;
            cs.set_high(); // Deselect device

            // Log progress every 10% for large areas
            if full_batches > 10 && (batch + 1) % (full_batches / 10) == 0 {
                let progress = ((batch + 1) * 100) / full_batches;
                debug!("Batch progress: {}%", progress);
            }
        }

        // Send remaining pixels if any
        if remaining_pixels > 0 {
            debug!("Sending remaining {} pixels", remaining_pixels);
            let remaining_bytes = remaining_pixels * 2;
            dc.set_high(); // Data mode
            cs.set_low();  // Select device
            let _ = spi.write(&batch_buffer[0..remaining_bytes]).await;
            cs.set_high(); // Deselect device
        }

        debug!("Area fill completed - {} pixels sent in batches", pixel_count);
    }





    info!("SPI communication functions ready");

    // Initialize GC9D01 using the extracted function
    initialize_gc9d01(&mut spi, &mut cs_pin, &mut dc_pin, &mut rst_pin).await;

    info!("Starting 90° Simple Checkerboard Test");
    info!("This test demonstrates a simple checkerboard pattern in 90° orientation");
    info!("Each block should be clearly separated with no overlaps");

    // RGB565 color definitions
    const BLACK: u16 = 0x0000;

    // Clear entire physical screen area first
    // Physical GC9D01 screen is 40x160 pixels, rotated 270° to appear as 160x40
    const PHYSICAL_WIDTH: u16 = 40;   // Physical screen width
    const PHYSICAL_HEIGHT: u16 = 160; // Physical screen height
    const LOGICAL_WIDTH: u16 = 160;   // Logical width after 270° rotation
    const LOGICAL_HEIGHT: u16 = 40;   // Logical height after 270° rotation

    info!("Clearing entire logical screen area ({}x{}) with black...", LOGICAL_WIDTH, LOGICAL_HEIGHT);
    fill_area_with_color_90(&mut spi, &mut cs_pin, &mut dc_pin, 0, 0, LOGICAL_WIDTH - 1, LOGICAL_HEIGHT - 1, BLACK).await;
    info!("Entire logical screen area cleared with black");

    Timer::after_secs(2).await;

    // Simple checkerboard test with correct 90° coordinate transformation
    info!("Starting simple checkerboard test (90° orientation with coordinate transformation)");

    loop {
        // Clear screen first
        info!("Clearing logical screen with black...");
        fill_area_with_color_90(&mut spi, &mut cs_pin, &mut dc_pin, 0, 0, LOGICAL_WIDTH - 1, LOGICAL_HEIGHT - 1, 0x0000).await;
        Timer::after_secs(1).await;

        // Draw simple checkerboard with no overlaps for 160x40 logical screen
        info!("Drawing 16x4 checkerboard with 10x10 pixel blocks (90° orientation with coordinate transformation)");

        // RGB565 colors
        const RED: u16 = 0xF800;
        const GREEN: u16 = 0x07E0;
        const BLUE: u16 = 0x001F;
        const YELLOW: u16 = 0xFFE0;
        const MAGENTA: u16 = 0xF81F;
        const CYAN: u16 = 0x07FF;
        const WHITE: u16 = 0xFFFF;
        const BLACK: u16 = 0x0000;

        let colors = [RED, GREEN, BLUE, YELLOW, MAGENTA, CYAN, WHITE, BLACK];

        // Use 10x10 pixel blocks for 160x40 logical screen
        let block_size: u16 = 10; // 10x10 pixel blocks
        let blocks_x: u16 = 16;   // 16 blocks horizontally (160 pixels)
        let blocks_y: u16 = 4;    // 4 blocks vertically (40 pixels)
        let start_x: u16 = 0;     // Start from edge
        let start_y: u16 = 0;     // Start from edge

        info!("Drawing {}x{} checkerboard with {} pixel blocks", blocks_x, blocks_y, block_size);

        for row in 0..blocks_y {
            for col in 0..blocks_x {
                // Simple alternating pattern
                let color_index = ((row + col) as usize) % colors.len();
                let color = colors[color_index];

                // Calculate exact logical pixel coordinates with no overlap
                let x_start = start_x + col * block_size;
                let x_end = x_start + block_size - 1;
                let y_start = start_y + row * block_size;
                let y_end = y_start + block_size - 1;

                info!("Block ({},{}) -> logical pixels ({},{}) to ({},{}) color 0x{:04X}",
                      row, col, x_start, y_start, x_end, y_end, color);

                // Ensure we don't go out of logical bounds (160x40 screen)
                if x_end < LOGICAL_WIDTH && y_end < LOGICAL_HEIGHT {
                    fill_area_with_color_90(&mut spi, &mut cs_pin, &mut dc_pin,
                                           x_start, y_start, x_end, y_end, color).await;
                    Timer::after_millis(100).await; // Shorter delay for smaller blocks
                }
            }
        }

        info!("Simple checkerboard completed (90° orientation with coordinate transformation) - 16x4 blocks on 160x40 logical screen");
        Timer::after_secs(3).await;
    }
}
