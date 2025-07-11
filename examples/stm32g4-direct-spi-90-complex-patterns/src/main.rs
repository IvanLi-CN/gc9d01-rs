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

// Screen dimensions
const LOGICAL_WIDTH: u16 = 160;   // Logical width after 270° rotation
const LOGICAL_HEIGHT: u16 = 40;   // Logical height after 270° rotation
const SCREEN_PIXELS: usize = (LOGICAL_WIDTH as usize) * (LOGICAL_HEIGHT as usize);

// Physical chunk buffer to avoid memory issues
// 物理屏幕布局：40宽×160高，我们按物理布局分块
const PHYSICAL_CHUNK_HEIGHT: usize = 8;  // Process 8 physical lines at a time for better efficiency
const PHYSICAL_CHUNK_WIDTH: usize = 40;   // Physical screen width
const CHUNK_PIXELS: usize = PHYSICAL_CHUNK_WIDTH * PHYSICAL_CHUNK_HEIGHT;
static mut CHUNK_BUFFER: [u16; CHUNK_PIXELS] = [0; CHUNK_PIXELS];

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
    info!("Starting GC9D01 90° Complex Patterns Test Firmware");

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
    spi_config.frequency = Hertz(32_000_000); // 32MHz SPI frequency for maximum speed

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

    // Chunk buffer manipulation functions
    fn clear_chunk_buffer(color: u16) {
        unsafe {
            for i in 0..CHUNK_PIXELS {
                CHUNK_BUFFER[i] = color;
            }
        }
    }

    // Convert HSV to RGB565
    fn hsv_to_rgb565(h: f32, s: f32, v: f32) -> u16 {
        let c = v * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;

        let (r_prime, g_prime, b_prime) = if h < 60.0 {
            (c, x, 0.0)
        } else if h < 120.0 {
            (x, c, 0.0)
        } else if h < 180.0 {
            (0.0, c, x)
        } else if h < 240.0 {
            (0.0, x, c)
        } else if h < 300.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };

        let r = ((r_prime + m) * 255.0) as u8;
        let g = ((g_prime + m) * 255.0) as u8;
        let b = ((b_prime + m) * 255.0) as u8;

        // Convert to RGB565: RRRRR GGGGGG BBBBB
        let r5 = (r >> 3) as u16;
        let g6 = (g >> 2) as u16;
        let b5 = (b >> 3) as u16;

        (r5 << 11) | (g6 << 5) | b5
    }

    fn set_pixel_in_chunk(logical_x: u16, logical_y: u16, color: u16, physical_chunk_start_y: u16) {
        // 检查逻辑坐标是否有效
        if logical_x < LOGICAL_WIDTH && logical_y < LOGICAL_HEIGHT {
            // 对于90°+180°旋转：logical(x,y) -> physical(39-y, 159-x)
            let physical_x = 39 - logical_y;
            let physical_y = 159 - logical_x;

            // 检查物理坐标是否在当前物理chunk范围内
            if physical_y >= physical_chunk_start_y && physical_y < physical_chunk_start_y + PHYSICAL_CHUNK_HEIGHT as u16 {
                // 计算在chunk buffer中的位置
                let chunk_physical_y = physical_y - physical_chunk_start_y;
                let index = (chunk_physical_y as usize) * PHYSICAL_CHUNK_WIDTH + (physical_x as usize);

                if index < CHUNK_PIXELS {
                    unsafe {
                        CHUNK_BUFFER[index] = color;
                    }
                }
            }
        }
    }

    fn fill_rect_in_chunk(logical_x0: u16, logical_y0: u16, logical_x1: u16, logical_y1: u16, color: u16, physical_chunk_start_y: u16) {
        for logical_y in logical_y0..=logical_y1 {
            for logical_x in logical_x0..=logical_x1 {
                set_pixel_in_chunk(logical_x, logical_y, color, physical_chunk_start_y);
            }
        }
    }

    // Flush physical chunk buffer to display
    async fn flush_chunk_buffer(
        spi: &mut Spi<'_, Async>,
        cs: &mut Output<'_>,
        dc: &mut Output<'_>,
        physical_chunk_start_y: u16
    ) {


        // Set address window for this physical chunk area
        let physical_x0 = 0;
        let physical_y0 = physical_chunk_start_y;
        let physical_x1 = 39;  // Physical width - 1
        let physical_y1 = physical_chunk_start_y + PHYSICAL_CHUNK_HEIGHT as u16 - 1;

        set_address_window(spi, cs, dc, physical_x0, physical_y0, physical_x1, physical_y1).await;
        start_memory_write(spi, cs, dc).await;

        // Convert chunk buffer to bytes and send
        const CHUNK_BYTES: usize = CHUNK_PIXELS * 2;
        let mut chunk_bytes = [0u8; CHUNK_BYTES];

        unsafe {
            for i in 0..CHUNK_PIXELS {
                let pixel_color = CHUNK_BUFFER[i];
                let byte_index = i * 2;
                if byte_index + 1 < CHUNK_BYTES {
                    chunk_bytes[byte_index] = (pixel_color >> 8) as u8;     // High byte
                    chunk_bytes[byte_index + 1] = (pixel_color & 0xFF) as u8; // Low byte
                }
            }
        }

        // Send chunk data in batches
        const BATCH_SIZE: usize = 1024; // Send in larger batches for better efficiency
        dc.set_high(); // Data mode
        cs.set_low();  // Select device

        let mut bytes_sent = 0;
        while bytes_sent < CHUNK_BYTES {
            let remaining = CHUNK_BYTES - bytes_sent;
            let batch_size = remaining.min(BATCH_SIZE);
            let _ = spi.write(&chunk_bytes[bytes_sent..bytes_sent + batch_size]).await;
            bytes_sent += batch_size;
        }

        cs.set_high(); // Deselect device
    }

    // Render entire screen using physical chunked approach
    async fn render_with_chunked_buffer<F>(
        spi: &mut Spi<'_, Async>,
        cs: &mut Output<'_>,
        dc: &mut Output<'_>,
        mut render_fn: F
    )
    where
        F: FnMut(u16) // Closure that renders to logical screen, given physical chunk start y
    {
        // Process physical screen in chunks (40x160 physical screen)
        for physical_chunk_start_y in (0..160).step_by(PHYSICAL_CHUNK_HEIGHT) {

            // Clear chunk buffer
            clear_chunk_buffer(0x0000); // Black

            // Render content for entire logical screen, but only pixels that map to this physical chunk will be stored
            render_fn(physical_chunk_start_y as u16);

            // Flush physical chunk to display
            flush_chunk_buffer(spi, cs, dc, physical_chunk_start_y as u16).await;
        }
    }





    info!("SPI communication functions ready");

    // Initialize GC9D01 using the extracted function
    initialize_gc9d01(&mut spi, &mut cs_pin, &mut dc_pin, &mut rst_pin).await;

    info!("Starting 90° Complex Patterns Test");
    info!("This test demonstrates complex patterns in 90° orientation using direct SPI");
    info!("Configuration: 160×40 logical screen with coordinate transformation");

    // RGB565 color definitions
    const RED: u16 = 0xF800;
    const GREEN: u16 = 0x07E0;
    const BLUE: u16 = 0x001F;
    const YELLOW: u16 = 0xFFE0;
    const MAGENTA: u16 = 0xF81F;
    const CYAN: u16 = 0x07FF;
    const WHITE: u16 = 0xFFFF;
    const BLACK: u16 = 0x0000;
    const ORANGE: u16 = 0xFD20;
    const PURPLE: u16 = 0x8010;

    let colors = [RED, GREEN, BLUE, YELLOW, MAGENTA, CYAN, WHITE, BLACK, ORANGE, PURPLE];

    // Clear entire screen using chunked buffer
    info!("Clearing entire logical screen area ({}x{}) with black...", LOGICAL_WIDTH, LOGICAL_HEIGHT);
    render_with_chunked_buffer(&mut spi, &mut cs_pin, &mut dc_pin, |_physical_chunk_start_y| {
        clear_chunk_buffer(BLACK);
    }).await;
    info!("Entire logical screen area cleared with black");

    Timer::after_secs(2).await;

    loop {
        // Test 0: Basic fill test to verify display is working
        info!("Test 0: Basic Fill Test");

        // Fill with red
        info!("Filling screen with RED...");
        render_with_chunked_buffer(&mut spi, &mut cs_pin, &mut dc_pin, |_physical_chunk_start_y| {
            clear_chunk_buffer(RED);
        }).await;
        Timer::after_secs(2).await;

        // Fill with green
        info!("Filling screen with GREEN...");
        render_with_chunked_buffer(&mut spi, &mut cs_pin, &mut dc_pin, |_physical_chunk_start_y| {
            clear_chunk_buffer(GREEN);
        }).await;
        Timer::after_secs(2).await;

        // Fill with blue
        info!("Filling screen with BLUE...");
        render_with_chunked_buffer(&mut spi, &mut cs_pin, &mut dc_pin, |_physical_chunk_start_y| {
            clear_chunk_buffer(BLUE);
        }).await;
        Timer::after_secs(2).await;

        // Pattern 1: Complex Checkerboard with Multiple Colors
        info!("Pattern 1: Complex Multi-Color Checkerboard (90°)");

        render_with_chunked_buffer(&mut spi, &mut cs_pin, &mut dc_pin, |physical_chunk_start_y| {
            // Create a complex checkerboard pattern for 160×40 logical screen
            let block_width = 20;  // 160 / 8 = 20 pixels wide
            let block_height = 20; // 40 / 2 = 20 pixels high
            let blocks_x = 8;      // 8 blocks across (160 pixels width)
            let blocks_y = 2;      // 2 blocks down (40 pixels height)

            for row in 0..blocks_y {
                for col in 0..blocks_x {
                    let color_index = ((row * blocks_x + col) as usize) % colors.len();
                    let color = colors[color_index];

                    let x = col * block_width;
                    let y = row * block_height;

                    // Fill the block area in chunk buffer
                    fill_rect_in_chunk(x, y, x + block_width - 1, y + block_height - 1, color, physical_chunk_start_y);
                }
            }
        }).await;

        info!("Complex checkerboard pattern completed");
        Timer::after_secs(5).await;

        // Pattern 2: 10x10 Color Checkerboard
        info!("Pattern 2: 10x10 Color Checkerboard (90°)");

        render_with_chunked_buffer(&mut spi, &mut cs_pin, &mut dc_pin, |physical_chunk_start_y| {
            // Create a 10x10 color checkerboard for 160×40 logical screen
            let block_width = 10;
            let block_height = 10;
            let blocks_x = 16;     // 160 / 10 = 16 blocks across
            let blocks_y = 4;      // 40 / 10 = 4 blocks down

            for row in 0..blocks_y {
                for col in 0..blocks_x {
                    let color_index = ((row + col) as usize) % colors.len();
                    let color = colors[color_index];

                    let x = col * block_width;
                    let y = row * block_height;

                    // Fill the block area in chunk buffer
                    fill_rect_in_chunk(x, y, x + block_width - 1, y + block_height - 1, color, physical_chunk_start_y);
                }
            }
        }).await;

        info!("Pattern 2 completed");
        Timer::after_secs(5).await;

        // Pattern 3: Gradient Stripes
        info!("Pattern 3: Gradient Color Stripes (90°)");

        render_with_chunked_buffer(&mut spi, &mut cs_pin, &mut dc_pin, |physical_chunk_start_y| {
            // Create vertical stripes with gradient effect for 160×40 logical screen
            let stripe_width = 16; // 160 / 10 = 16 pixels per stripe
            let stripes = 10;

            for stripe in 0..stripes {
                let x = stripe * stripe_width;

                // Create gradient within each stripe (height: 40)
                for y in 0..LOGICAL_HEIGHT {
                    let intensity = (y as f32 / 39.0 * 31.0) as u16;
                    let gradient_color = match stripe % 3 {
                        0 => intensity << 11, // Red gradient (bits 15-11)
                        1 => intensity << 6,  // Green gradient (bits 10-5, but intensity*2 for 6-bit)
                        _ => intensity,       // Blue gradient (bits 4-0)
                    };

                    // Fill one horizontal line of the stripe in chunk buffer
                    fill_rect_in_chunk(x, y, x + stripe_width - 1, y, gradient_color, physical_chunk_start_y);
                }
            }
        }).await;

        info!("Pattern 3 completed");
        Timer::after_secs(5).await;

        // Pattern 4: Rainbow Gradient with Saturation
        info!("Pattern 4: Rainbow Gradient with Saturation (90°)");

        render_with_chunked_buffer(&mut spi, &mut cs_pin, &mut dc_pin, |physical_chunk_start_y| {
            // Create rainbow gradient: long edge (160px) = hue, short edge (40px) = saturation
            for logical_y in 0..LOGICAL_HEIGHT {
                for logical_x in 0..LOGICAL_WIDTH {
                    // Hue varies along the long edge (160 pixels)
                    let hue = (logical_x as f32 / LOGICAL_WIDTH as f32) * 360.0;

                    // Saturation varies along the short edge (40 pixels): 0% to 100%
                    let saturation = logical_y as f32 / (LOGICAL_HEIGHT - 1) as f32;

                    // Convert HSV to RGB565
                    let rgb = hsv_to_rgb565(hue, saturation, 1.0); // Full brightness

                    set_pixel_in_chunk(logical_x, logical_y, rgb, physical_chunk_start_y);
                }
            }
        }).await;

        info!("Pattern 4 completed");
        Timer::after_secs(5).await;

        // Pattern 5: Concentric Rectangles
        info!("Pattern 5: Concentric Rectangles (90°)");

        render_with_chunked_buffer(&mut spi, &mut cs_pin, &mut dc_pin, |physical_chunk_start_y| {
            // Draw concentric rectangles from outside to inside
            for layer in 0..5 {
                let color = colors[layer % colors.len()];

                // Top and bottom borders for 160×40 logical screen
                for border_y in [layer * 4, 39 - layer * 4] {
                    if border_y < 40 {
                        let start_x = layer * 16;
                        let end_x = 159 - layer * 16;
                        if start_x <= end_x && end_x < 160 {
                            fill_rect_in_chunk(start_x as u16, border_y as u16, end_x as u16, border_y as u16, color, physical_chunk_start_y);
                        }
                    }
                }

                // Left and right borders for 160×40 logical screen
                for border_x in [layer * 16, 159 - layer * 16] {
                    if border_x < 160 {
                        let start_y = layer * 4;
                        let end_y = 39 - layer * 4;
                        if start_y <= end_y && end_y < 40 {
                            fill_rect_in_chunk(border_x as u16, start_y as u16, border_x as u16, end_y as u16, color, physical_chunk_start_y);
                        }
                    }
                }
            }
        }).await;

        info!("Pattern 5 completed");
        Timer::after_secs(5).await;

        // Pattern 6: Diagonal Lines Pattern
        info!("Pattern 6: Diagonal Lines Pattern (90°)");

        render_with_chunked_buffer(&mut spi, &mut cs_pin, &mut dc_pin, |physical_chunk_start_y| {
            // Draw diagonal lines across the screen for 160×40 logical screen
            for line in 0..20 {
                let color = colors[line % colors.len()];
                let spacing = 8;

                // Draw diagonal line from top-left to bottom-right (160×40)
                for step in 0..200 {
                    let x = (step + line * spacing) % 160; // screen width
                    let y = (step * 40 / 160) % 40;        // screen height

                    if x < 160 && y < 40 {
                        set_pixel_in_chunk(x as u16, y as u16, color, physical_chunk_start_y);
                    }
                }
            }
        }).await;

        info!("Pattern 6 completed");
        Timer::after_secs(5).await;


    }
}
