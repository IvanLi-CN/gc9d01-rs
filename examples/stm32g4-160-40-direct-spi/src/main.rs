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

// Screen orientation enumeration for GC9D01
#[derive(Clone, Copy, Debug)]
pub enum ScreenOrientation {
    Portrait = 0x00,        // 0°
    Landscape = 0x60,       // 90°
    PortraitSwapped = 0x80, // 180°
    LandscapeSwapped = 0xA0, // 270°
}

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
    info!("Starting GC9D01 Direct SPI Test Firmware");

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

    async fn write_data(spi: &mut Spi<'_, Async>, cs: &mut Output<'_>, dc: &mut Output<'_>, data: u8) {
        debug!("Sending data: 0x{:02X}", data);
        dc.set_high(); // Data mode
        cs.set_low();  // Select device
        let _ = spi.write(&[data]).await;
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



    // Fill area with solid color - optimized batch version with orientation support
    async fn fill_area_with_color(
        spi: &mut Spi<'_, Async>,
        cs: &mut Output<'_>,
        dc: &mut Output<'_>,
        x0: u16, y0: u16, x1: u16, y1: u16,
        color: u16,
        orientation: ScreenOrientation
    ) {
        let width = x1 - x0 + 1;
        let height = y1 - y0 + 1;
        let pixel_count = (width as u32) * (height as u32);

        debug!("Original coordinates: ({},{}) to ({},{}) in orientation {:?}",
               x0, y0, x1, y1, orientation);

        // Transform coordinates based on orientation to handle coordinate system changes
        // The display is initialized with a fixed orientation, we only transform coordinates
        // For 90° and 270°, we need to transpose coordinates to fit the rotated coordinate system
        // For 0° and 180°, we keep the same coordinates but the content arrangement differs

        let (final_x0, final_y0, final_x1, final_y1) = match orientation {
            ScreenOrientation::Portrait => {
                // 0°: No transformation needed
                (x0, y0, x1, y1)
            },
            ScreenOrientation::Landscape => {
                // 90°: Transpose coordinates for landscape orientation
                // x,y -> y,x to fit the rotated coordinate system
                (y0, x0, y1, x1)
            },
            ScreenOrientation::PortraitSwapped => {
                // 180°: Keep same coordinates, content will be arranged differently
                // The rotation effect is achieved by changing the content layout, not coordinates
                (x0, y0, x1, y1)
            },
            ScreenOrientation::LandscapeSwapped => {
                // 270°: Transpose coordinates for landscape orientation
                // x,y -> y,x to fit the rotated coordinate system
                (y0, x0, y1, x1)
            },
        };

        debug!("Transformed coordinates: ({},{}) to ({},{}) for orientation {:?}",
               final_x0, final_y0, final_x1, final_y1, orientation);

        set_address_window(spi, cs, dc, final_x0, final_y0, final_x1, final_y1).await;
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

    // Display test functionality - 8 color bars
    info!("Starting display test - 8 color bars");

    // RGB565 color definitions for the 8 colors
    const MAGENTA: u16 = 0xF81F;   // 品红 (Red + Blue)
    const RED: u16 = 0xF800;       // 红
    const YELLOW: u16 = 0xFFE0;    // 黄 (Red + Green)
    const GREEN: u16 = 0x07E0;     // 绿
    const CYAN: u16 = 0x07FF;      // 青 (Green + Blue)
    const BLUE: u16 = 0x001F;      // 蓝
    const PURPLE: u16 = 0x8010;    // 紫 (Dark Magenta)
    const WHITE: u16 = 0xFFFF;     // 白

    // Screen dimensions after 90° rotation: 40x160 (was 160x40)
    const DISPLAY_WIDTH: u16 = 40;
    const DISPLAY_HEIGHT: u16 = 160;

    // Vertical bar dimensions: 8 bars across full display width, 20 pixels high each
    const BAR_WIDTH: u16 = DISPLAY_WIDTH / 8;  // Each bar is 5 pixels wide (40/8=5)
    const BAR_HEIGHT: u16 = 40;                // Each bar is 40 pixels high (more visible)

    // Array of colors to display (first row)
    let colors = [MAGENTA, RED, YELLOW, GREEN, CYAN, BLUE, PURPLE, WHITE];
    let color_names = ["品红", "红", "黄", "绿", "青", "蓝", "紫", "白"];

    // Array of grayscale colors (second row)
    let grayscales = [
        0x0000, // Black
        0x2104, // Dark gray
        0x4208, // Medium dark gray
        0x630C, // Medium gray
        0x8410, // Medium light gray
        0xA514, // Light gray
        0xC618, // Very light gray
        0xFFFF, // White
    ];
    let gray_names = ["黑", "深灰1", "深灰2", "中灰", "浅灰1", "浅灰2", "很浅灰", "白"];

    // First, fill entire controllable area with black to clear any previous content
    // GC9D01 has 360x360 internal GRAM, use maximum addressable range
    const BLACK: u16 = 0x0000;
    const MAX_WIDTH: u16 = 360;
    const MAX_HEIGHT: u16 = 360;

    info!("Clearing entire controllable area ({}x{}) with black...", MAX_WIDTH, MAX_HEIGHT);
    fill_area_with_color(
        &mut spi, &mut cs_pin, &mut dc_pin,
        0, 0, MAX_WIDTH - 1, MAX_HEIGHT - 1,
        BLACK,
        ScreenOrientation::Portrait  // Use default orientation for clearing
    ).await;
    info!("Entire controllable area cleared with black");

    // Wait a moment to see the black screen
    Timer::after_millis(1000).await;

    // Two-column test pattern (transposed):
    // Column 1: 8 colors (品红、红、黄、绿、青、蓝、紫、白) - each 20x20
    // Column 2: 8 grayscale levels - each 20x20
    info!("Creating two-column test pattern (20x20 each, transposed)");

    // Clear screen first
    fill_area_with_color(&mut spi, &mut cs_pin, &mut dc_pin, 0, 0, 359, 359, BLACK, ScreenOrientation::Portrait).await;
    Timer::after_millis(500).await;

    // Column 1: Define colors: 品红、红、黄、绿、青、蓝、紫、白
    let colors = [
        MAGENTA,  // 品红 0xF81F
        RED,      // 红   0xF800
        YELLOW,   // 黄   0xFFE0
        GREEN,    // 绿   0x07E0
        0x07FF,   // 青   Cyan
        BLUE,     // 蓝   0x001F
        0x781F,   // 紫   Purple
        WHITE,    // 白   0xFFFF
    ];

    let color_names = ["品红", "红", "黄", "绿", "青", "蓝", "紫", "白"];

    // Draw first column: 8 color blocks, each 20 pixels wide, 20 pixels high
    info!("Drawing first column: 8 colors");
    for (i, (&color, &name)) in colors.iter().zip(color_names.iter()).enumerate() {
        let x_start = 0;              // First column starts at x=0
        let x_end = 19;               // First column is 20 pixels wide
        let y_start = i as u16 * 20;  // Each block starts at i*20
        let y_end = y_start + 19;     // Each block is 20 pixels high

        info!("Col 1, block {}: {} at x={}-{}, y={}-{}", i+1, name, x_start, x_end, y_start, y_end);
        fill_area_with_color(&mut spi, &mut cs_pin, &mut dc_pin, x_start, y_start, x_end, y_end, color, ScreenOrientation::Portrait).await;
        Timer::after_millis(300).await;
    }

    // Column 2: Define 8 grayscale levels (from black to white)
    let grayscale = [
        0x0000,  // Black
        0x2104,  // Dark gray
        0x4208,  // Medium dark gray
        0x630C,  // Medium gray
        0x8410,  // Medium light gray
        0xA514,  // Light gray
        0xC618,  // Very light gray
        0xFFFF,  // White
    ];

    // Draw second column: 8 grayscale blocks, each 20 pixels wide, 20 pixels high
    info!("Drawing second column: 8 grayscale levels");
    for (i, &gray_color) in grayscale.iter().enumerate() {
        let x_start = 20;             // Second column starts at x=20
        let x_end = 39;               // Second column ends at x=39
        let y_start = i as u16 * 20;  // Each block starts at i*20
        let y_end = y_start + 19;     // Each block is 20 pixels high

        info!("Col 2, block {}: Gray 0x{:04X} at x={}-{}, y={}-{}", i+1, gray_color, x_start, x_end, y_start, y_end);
        fill_area_with_color(&mut spi, &mut cs_pin, &mut dc_pin, x_start, y_start, x_end, y_end, gray_color, ScreenOrientation::Portrait).await;
        Timer::after_millis(300).await;
    }

    info!("Two-column test pattern complete (transposed)!");
    info!("Column 1 (x=0-19): 8 colors vertically, each 20x20 pixels:");
    info!("  1. 品红 (Magenta) - 0xF81F at y=0-19");
    info!("  2. 红 (Red) - 0xF800 at y=20-39");
    info!("  3. 黄 (Yellow) - 0xFFE0 at y=40-59");
    info!("  4. 绿 (Green) - 0x07E0 at y=60-79");
    info!("  5. 青 (Cyan) - 0x07FF at y=80-99");
    info!("  6. 蓝 (Blue) - 0x001F at y=100-119");
    info!("  7. 紫 (Purple) - 0x781F at y=120-139");
    info!("  8. 白 (White) - 0xFFFF at y=140-159");
    info!("Column 2 (x=20-39): 8 grayscale levels vertically, each 20x20 pixels:");
    info!("  1. Black - 0x0000 at y=0-19");
    info!("  2. Dark Gray - 0x2104 at y=20-39");
    info!("  3. Medium Dark Gray - 0x4208 at y=40-59");
    info!("  4. Medium Gray - 0x630C at y=60-79");
    info!("  5. Medium Light Gray - 0x8410 at y=80-99");
    info!("  6. Light Gray - 0xA514 at y=100-119");
    info!("  7. Very Light Gray - 0xC618 at y=120-139");
    info!("  8. White - 0xFFFF at y=140-159");
    info!("Total pattern size: 40x160 pixels (transposed)");
    info!("This tests color accuracy, grayscale gradient, and positioning!");

    // Wait before demonstrating orientation changes
    Timer::after_secs(3).await;

    // Demonstrate different screen orientations in continuous loop
    info!("Starting continuous orientation demonstration...");

    let orientations = [
        (ScreenOrientation::Portrait, "Portrait (0°)"),
        (ScreenOrientation::Landscape, "Landscape (90°)"),
        (ScreenOrientation::PortraitSwapped, "Portrait Swapped (180°)"),
        (ScreenOrientation::LandscapeSwapped, "Landscape Swapped (270°)"),
    ];

    info!("All rendering tests completed successfully!");
    info!("Starting continuous orientation loop with 10-second intervals...");

    // Continuous loop to demonstrate orientations
    let mut cycle_count = 0;
    loop {
        cycle_count += 1;
        info!("=== Starting orientation cycle {} ===", cycle_count);

        for (orientation, name) in orientations.iter() {
            info!("Testing orientation: {}", name);

            // Clear screen with black
            fill_area_with_color(&mut spi, &mut cs_pin, &mut dc_pin, 0, 0, 359, 359, BLACK, *orientation).await;
            Timer::after_millis(500).await;

            // Render test pattern based on orientation
            match orientation {
                ScreenOrientation::Portrait => {
                    // Portrait (0°): normal vertical layout (2 columns, 8 rows each)
                    info!("Drawing first column: 8 colors in orientation {}", name);
                    for (i, (&color, &color_name)) in colors.iter().zip(color_names.iter()).enumerate() {
                        let x_start = 0;              // First column starts at x=0
                        let x_end = 19;               // First column is 20 pixels wide
                        let y_start = i as u16 * 20;  // Each block starts at i*20
                        let y_end = y_start + 19;     // Each block is 20 pixels high

                        info!("Col 1, block {}: {} at x={}-{}, y={}-{} ({})", i+1, color_name, x_start, x_end, y_start, y_end, name);
                        fill_area_with_color(&mut spi, &mut cs_pin, &mut dc_pin, x_start, y_start, x_end, y_end, color, *orientation).await;
                        Timer::after_millis(100).await;
                    }

                    info!("Drawing second column: 8 grayscale levels in orientation {}", name);
                    for (i, &gray_color) in grayscale.iter().enumerate() {
                        let x_start = 20;             // Second column starts at x=20
                        let x_end = 39;               // Second column ends at x=39
                        let y_start = i as u16 * 20;  // Each block starts at i*20
                        let y_end = y_start + 19;     // Each block is 20 pixels high

                        info!("Col 2, block {}: Gray 0x{:04X} at x={}-{}, y={}-{} ({})", i+1, gray_color, x_start, x_end, y_start, y_end, name);
                        fill_area_with_color(&mut spi, &mut cs_pin, &mut dc_pin, x_start, y_start, x_end, y_end, gray_color, *orientation).await;
                        Timer::after_millis(100).await;
                    }
                },
                ScreenOrientation::PortraitSwapped => {
                    // Portrait Swapped (180°): true 180° rotation - both columns and rows are flipped
                    // First column becomes second column at bottom, second column becomes first column at bottom
                    info!("Drawing first column (180° rotated): 8 colors in orientation {}", name);
                    for (i, (&color, &color_name)) in colors.iter().zip(color_names.iter()).enumerate() {
                        // 180° rotation: first column (x=0-19) becomes second column (x=20-39)
                        // and rows are flipped: row 0 becomes row 7, row 1 becomes row 6, etc.
                        let x_start = 20;                        // Rotated to second column
                        let x_end = 39;                          // Second column is 20 pixels wide
                        let y_start = (7 - i) as u16 * 20;       // Flip the row order
                        let y_end = y_start + 19;                // Each block is 20 pixels high

                        info!("Col 1 (180°), block {}: {} at x={}-{}, y={}-{} ({})", i+1, color_name, x_start, x_end, y_start, y_end, name);
                        fill_area_with_color(&mut spi, &mut cs_pin, &mut dc_pin, x_start, y_start, x_end, y_end, color, *orientation).await;
                        Timer::after_millis(100).await;
                    }

                    info!("Drawing second column (180° rotated): 8 grayscale levels in orientation {}", name);
                    for (i, &gray_color) in grayscale.iter().enumerate() {
                        // 180° rotation: second column (x=20-39) becomes first column (x=0-19)
                        // and rows are flipped: row 0 becomes row 7, row 1 becomes row 6, etc.
                        let x_start = 0;                         // Rotated to first column
                        let x_end = 19;                          // First column is 20 pixels wide
                        let y_start = (7 - i) as u16 * 20;       // Flip the row order
                        let y_end = y_start + 19;                // Each block is 20 pixels high

                        info!("Col 2 (180°), block {}: Gray 0x{:04X} at x={}-{}, y={}-{} ({})", i+1, gray_color, x_start, x_end, y_start, y_end, name);
                        fill_area_with_color(&mut spi, &mut cs_pin, &mut dc_pin, x_start, y_start, x_end, y_end, gray_color, *orientation).await;
                        Timer::after_millis(100).await;
                    }
                },
                ScreenOrientation::Landscape => {
                    // Landscape (90°): horizontal layout (2 rows, 8 columns each)
                    info!("Drawing first row: 8 colors in orientation {}", name);
                    for (i, (&color, &color_name)) in colors.iter().zip(color_names.iter()).enumerate() {
                        let x_start = i as u16 * 20;  // Each block starts at i*20
                        let x_end = x_start + 19;     // Each block is 20 pixels wide
                        let y_start = 0;              // First row starts at y=0
                        let y_end = 19;               // First row is 20 pixels high

                        info!("Row 1, block {}: {} at x={}-{}, y={}-{} ({})", i+1, color_name, x_start, x_end, y_start, y_end, name);
                        fill_area_with_color(&mut spi, &mut cs_pin, &mut dc_pin, x_start, y_start, x_end, y_end, color, *orientation).await;
                        Timer::after_millis(100).await;
                    }

                    info!("Drawing second row: 8 grayscale levels in orientation {}", name);
                    for (i, &gray_color) in grayscale.iter().enumerate() {
                        let x_start = i as u16 * 20;  // Each block starts at i*20
                        let x_end = x_start + 19;     // Each block is 20 pixels wide
                        let y_start = 20;             // Second row starts at y=20
                        let y_end = 39;               // Second row is 20 pixels high

                        info!("Row 2, block {}: Gray 0x{:04X} at x={}-{}, y={}-{} ({})", i+1, gray_color, x_start, x_end, y_start, y_end, name);
                        fill_area_with_color(&mut spi, &mut cs_pin, &mut dc_pin, x_start, y_start, x_end, y_end, gray_color, *orientation).await;
                        Timer::after_millis(100).await;
                    }
                },
                ScreenOrientation::LandscapeSwapped => {
                    // Landscape Swapped (270°): horizontal layout but rotated 180° from 90°
                    // First row becomes second row from right to left, second row becomes first row from right to left
                    info!("Drawing first row (270° rotated): 8 colors in orientation {}", name);
                    for (i, (&color, &color_name)) in colors.iter().zip(color_names.iter()).enumerate() {
                        // 270° rotation: first row becomes second row, and columns are flipped
                        // Column 0 becomes column 7, column 1 becomes column 6, etc.
                        let x_start = (7 - i) as u16 * 20;  // Flip the column order
                        let x_end = x_start + 19;           // Each block is 20 pixels wide
                        let y_start = 20;                   // Rotated to second row
                        let y_end = 39;                     // Second row is 20 pixels high

                        info!("Row 1 (270°), block {}: {} at x={}-{}, y={}-{} ({})", i+1, color_name, x_start, x_end, y_start, y_end, name);
                        fill_area_with_color(&mut spi, &mut cs_pin, &mut dc_pin, x_start, y_start, x_end, y_end, color, *orientation).await;
                        Timer::after_millis(100).await;
                    }

                    info!("Drawing second row (270° rotated): 8 grayscale levels in orientation {}", name);
                    for (i, &gray_color) in grayscale.iter().enumerate() {
                        // 270° rotation: second row becomes first row, and columns are flipped
                        // Column 0 becomes column 7, column 1 becomes column 6, etc.
                        let x_start = (7 - i) as u16 * 20;  // Flip the column order
                        let x_end = x_start + 19;           // Each block is 20 pixels wide
                        let y_start = 0;                    // Rotated to first row
                        let y_end = 19;                     // First row is 20 pixels high

                        info!("Row 2 (270°), block {}: Gray 0x{:04X} at x={}-{}, y={}-{} ({})", i+1, gray_color, x_start, x_end, y_start, y_end, name);
                        fill_area_with_color(&mut spi, &mut cs_pin, &mut dc_pin, x_start, y_start, x_end, y_end, gray_color, *orientation).await;
                        Timer::after_millis(100).await;
                    }
                }
            }

            info!("Orientation {} completed - Two-column pattern rendered", name);

            // Wait 10 seconds before next orientation
            info!("Waiting 10 seconds before next orientation...");
            Timer::after_secs(10).await;
        }

        info!("=== Completed orientation cycle {} ===", cycle_count);
        info!("Starting next cycle in 2 seconds...");
        Timer::after_secs(2).await;
    }
}
