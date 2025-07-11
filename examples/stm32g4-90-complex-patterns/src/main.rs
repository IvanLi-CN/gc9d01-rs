#![no_std]
#![no_main]

use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice as EmbassySpiDevice;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::mode;
use embassy_stm32::spi::{Config as SpiConfig, Spi as Stm32Spi};
use embassy_stm32::time::Hertz;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embedded_graphics::{
    pixelcolor::Rgb565,
};
use micromath::F32Ext;
use gc9d01::{Config as DisplayDriverConfig, GC9D01, Orientation, Timer as Gc9d01Timer};
use static_cell::StaticCell;

use {defmt_rtt as _, panic_probe as _};

use defmt::*;

// Timer implementation for the GC9D01 driver
struct EmbassyDisplayTimer;

impl Gc9d01Timer for EmbassyDisplayTimer {
    async fn after_millis(milliseconds: u64) {
        embassy_time::Timer::after_millis(milliseconds).await
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Starting GC9D01 90° Complex Patterns Test using Library");

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
            // Main system clock at 170 MHz
            divr: Some(PllRDiv::DIV2),
        });
        config.rcc.mux.adc12sel = mux::Adcsel::SYS;
        config.rcc.sys = Sysclk::PLL1_R;
        config.rcc.mux.clk48sel = mux::Clk48sel::HSI48;
    }
    let p = embassy_stm32::init(config);

    info!("Hardware initialized successfully");

    // Configure GPIO pins for GC9D01
    let sck_pin = p.PB3;   // SPI Clock
    let mosi_pin = p.PA7;  // SPI MOSI (Master Out Slave In)
    let cs_pin = Output::new(p.PA4, Level::High, Speed::VeryHigh);  // Chip Select (active low)
    let dc_pin = Output::new(p.PB0, Level::Low, Speed::VeryHigh);   // Data/Command select
    let rst_pin = Output::new(p.PC4, Level::Low, Speed::VeryHigh);  // Reset (active low)

    // Configure SPI1 for GC9D01 communication
    let mut spi_config = SpiConfig::default();
    spi_config.frequency = Hertz(16_000_000); // 16MHz SPI frequency

    let spi = Stm32Spi::new_txonly(
        p.SPI1,
        sck_pin,
        mosi_pin,
        p.DMA1_CH1,
        spi_config,
    );

    info!("SPI and GPIO initialized successfully");

    // Create shared SPI bus
    static SPI_BUS: StaticCell<Mutex<CriticalSectionRawMutex, Stm32Spi<'static, mode::Async>>> = StaticCell::new();
    let spi_bus = SPI_BUS.init(Mutex::new(spi));

    // Create SPI device for the display
    let spi_device = EmbassySpiDevice::new(spi_bus, cs_pin);

    // Configure display for 90° orientation
    // Physical screen is 40×160, but we configure as 160×40 with Portrait orientation
    // This matches the working stm32g4 example configuration
    let display_config = DisplayDriverConfig {
        width: 160,  // Logical width (matches stm32g4 example)
        height: 40,  // Logical height (matches stm32g4 example)
        orientation: Orientation::Portrait, // Use Portrait like stm32g4 example
        rgb: false,
        inverted: false,
        dx: 0,
        dy: 0,
    };

    static DISPLAY_BUFFER_CELL: StaticCell<[u8; gc9d01::BUF_SIZE]> = StaticCell::new();
    let buffer_slice: &mut [u8] = DISPLAY_BUFFER_CELL.init([0; gc9d01::BUF_SIZE]);

    let mut display: GC9D01<
        '_,
        EmbassySpiDevice<
            'static,
            CriticalSectionRawMutex,
            Stm32Spi<'static, mode::Async>,
            Output<'static>,
        >,
        Output<'_>,
        Output<'_>,
        EmbassyDisplayTimer,
    > = GC9D01::new(display_config, spi_device, dc_pin, rst_pin, buffer_slice);

    info!("Initializing display...");
    match display.init().await {
        Ok(_) => info!("Display initialized successfully!"),
        Err(e) => error!("Display initialization failed: {:?}", e),
    }

    info!("Starting 90° Complex Patterns Test");
    info!("This test demonstrates complex patterns in 90° orientation using the library");
    info!("Configuration: 160×40 with Portrait orientation (matches stm32g4 example)");

    // RGB565 color definitions
    const RED: Rgb565 = Rgb565::new(31, 0, 0);
    const GREEN: Rgb565 = Rgb565::new(0, 63, 0);
    const BLUE: Rgb565 = Rgb565::new(0, 0, 31);
    const YELLOW: Rgb565 = Rgb565::new(31, 63, 0);
    const MAGENTA: Rgb565 = Rgb565::new(31, 0, 31);
    const CYAN: Rgb565 = Rgb565::new(0, 63, 31);
    const WHITE: Rgb565 = Rgb565::new(31, 63, 31);
    const BLACK: Rgb565 = Rgb565::new(0, 0, 0);
    const ORANGE: Rgb565 = Rgb565::new(31, 32, 0);
    const PURPLE: Rgb565 = Rgb565::new(16, 0, 31);

    let colors = [RED, GREEN, BLUE, YELLOW, MAGENTA, CYAN, WHITE, BLACK, ORANGE, PURPLE];

    loop {
        // Test 0: Basic fill test to verify display is working
        info!("Test 0: Basic Fill Test");

        // Fill with red
        info!("Filling screen with RED...");
        display.fill_color(RED).await.unwrap();
        embassy_time::Timer::after_secs(2).await;

        // Fill with green
        info!("Filling screen with GREEN...");
        display.fill_color(GREEN).await.unwrap();
        embassy_time::Timer::after_secs(2).await;

        // Fill with blue
        info!("Filling screen with BLUE...");
        display.fill_color(BLUE).await.unwrap();
        embassy_time::Timer::after_secs(2).await;

        // Pattern 1: Complex Checkerboard with Multiple Colors
        info!("Pattern 1: Complex Multi-Color Checkerboard (90°)");

        // Clear screen first
        display.fill_color(BLACK).await.unwrap();
        embassy_time::Timer::after_secs(1).await;

        // Create a complex checkerboard pattern for 160×40 logical screen
        // 8×2 blocks (20×20 pixels each) - similar to stm32g4 example
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

                // Create pixel data for this block
                let block_pixels = [color; (20 * 20) as usize];

                display.write_area(x, y, block_width, block_height, &block_pixels).await.unwrap();
            }
        }

        info!("Complex checkerboard pattern completed");
        embassy_time::Timer::after_secs(10).await;

        // Pattern 2: Gradient Stripes
        info!("Pattern 2: Gradient Color Stripes (90°)");

        display.fill_color(BLACK).await.unwrap();
        embassy_time::Timer::after_secs(1).await;

        // Create vertical stripes with gradient effect for 160×40 logical screen
        let stripe_width = 16; // 160 / 10 = 16 pixels per stripe
        let stripes = 10;

        for stripe in 0..stripes {
            let x = stripe * stripe_width;

            // Create gradient within each stripe (height: 40)
            for y in 0..40 {
                let intensity = (y as f32 / 39.0 * 31.0) as u8;
                let gradient_color = match stripe % 3 {
                    0 => Rgb565::new(intensity, 0, 0), // Red gradient
                    1 => Rgb565::new(0, intensity * 2, 0), // Green gradient
                    _ => Rgb565::new(0, 0, intensity), // Blue gradient
                };

                let line_pixels = [gradient_color; 16];
                display.write_area(x, y, stripe_width, 1, &line_pixels).await.unwrap();
            }
        }

        info!("Gradient stripes pattern completed");
        embassy_time::Timer::after_secs(10).await;

        // Pattern 3: Concentric Rectangles
        info!("Pattern 3: Concentric Rectangles (90°)");

        display.fill_color(BLACK).await.unwrap();
        embassy_time::Timer::after_secs(1).await;

        // Draw concentric rectangles from outside to inside
        for layer in 0..5 {
            let color = colors[layer % colors.len()];

            // Top and bottom borders for 160×40 logical screen
            for border_y in [layer * 4, 39 - layer * 4] {
                if border_y < 40 {
                    let start_x = layer * 16;
                    let end_x = 159 - layer * 16;
                    if start_x <= end_x && end_x < 160 {
                        let width = end_x - start_x + 1;
                        // Create line pixels array - use a reasonable max width
                        let line_pixels = [color; 160]; // Max possible width for 160×40 logical screen
                        let line_slice = &line_pixels[..width as usize];
                        display.write_area(start_x as u16, border_y as u16, width as u16, 1, line_slice).await.unwrap();
                    }
                }
            }

            // Left and right borders for 160×40 logical screen
            for border_x in [layer * 16, 159 - layer * 16] {
                if border_x < 160 {
                    let start_y = layer * 4;
                    let end_y = 39 - layer * 4;
                    if start_y <= end_y && end_y < 40 {
                        for y in start_y..=end_y {
                            let pixel = [color; 1];
                            display.write_area(border_x as u16, y as u16, 1, 1, &pixel).await.unwrap();
                        }
                    }
                }
            }
        }

        info!("Concentric rectangles pattern completed");
        embassy_time::Timer::after_secs(10).await;

        // Pattern 4: Diagonal Lines Pattern
        info!("Pattern 4: Diagonal Lines Pattern (90°)");

        display.fill_color(BLACK).await.unwrap();
        embassy_time::Timer::after_secs(1).await;

        // Draw diagonal lines across the screen for 160×40 logical screen
        for line in 0..20 {
            let color = colors[line % colors.len()];
            let spacing = 8;

            // Draw diagonal line from top-left to bottom-right (160×40)
            for step in 0..200 {
                let x = (step + line * spacing) % 160; // screen width
                let y = (step * 40 / 160) % 40;        // screen height

                if x < 160 && y < 40 {
                    let pixel = [color; 1];
                    display.write_area(x as u16, y as u16, 1, 1, &pixel).await.unwrap();
                }
            }
        }

        info!("Diagonal lines pattern completed");
        embassy_time::Timer::after_secs(10).await;

        // Pattern 5: Spiral Pattern
        info!("Pattern 5: Spiral Pattern (90°)");

        display.fill_color(BLACK).await.unwrap();
        embassy_time::Timer::after_secs(1).await;

        // Draw a spiral pattern for 160×40 logical screen
        let center_x = 80;  // center X (160/2)
        let center_y = 20;  // center Y (40/2)
        let max_radius = 20;

        for angle in 0..720 { // Two full rotations
            let radius = (angle as f32 / 720.0 * max_radius as f32) as u16;
            let rad = angle as f32 * 3.14159 / 180.0;
            let x = center_x as i32 + (radius as f32 * rad.cos()) as i32;
            let y = center_y as i32 + (radius as f32 * rad.sin()) as i32;

            if x >= 0 && x < 160 && y >= 0 && y < 40 {
                let color = colors[(angle / 72) % colors.len()];
                let pixel = [color; 1];
                display.write_area(x as u16, y as u16, 1, 1, &pixel).await.unwrap();
            }
        }

        info!("Spiral pattern completed");
        embassy_time::Timer::after_secs(10).await;
    }
}