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
    prelude::RgbColor,
};
use micromath::F32Ext;
use gc9d01::{Config as DisplayDriverConfig, GC9D01, Orientation, Timer as Gc9d01Timer};
use static_cell::StaticCell;

use {defmt_rtt as _, panic_probe as _};

use defmt::*;

// Screen dimensions - logical vs physical
const LOGICAL_WIDTH: usize = 160;   // Logical width after rotation
const LOGICAL_HEIGHT: usize = 40;   // Logical height after rotation
const PHYSICAL_WIDTH: usize = 40;   // Physical screen width
const PHYSICAL_HEIGHT: usize = 160; // Physical screen height
const SCREEN_PIXELS: usize = PHYSICAL_WIDTH * PHYSICAL_HEIGHT; // Frame buffer organized by physical layout

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



    // Create frame buffer for the new architecture
    static FRAME_BUFFER_CELL: StaticCell<[Rgb565; SCREEN_PIXELS]> = StaticCell::new();
    let frame_buffer: &mut [Rgb565] = FRAME_BUFFER_CELL.init([Rgb565::BLACK; SCREEN_PIXELS]);

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
    > = GC9D01::new(display_config, spi_device, dc_pin, rst_pin, frame_buffer);

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

    // Convert HSV to RGB565
    fn hsv_to_rgb565(h: f32, s: f32, v: f32) -> Rgb565 {
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

        // Convert to RGB565
        let r5 = r >> 3;
        let g6 = g >> 2;
        let b5 = b >> 3;

        Rgb565::new(r5, g6, b5)
    }

    loop {
        // Test 0: Basic fill test to verify display is working
        info!("Test 0: Basic Fill Test");

        // Fill with red
        info!("Filling screen with RED...");
        display.fill_color(RED);
        display.flush().await.unwrap();
        embassy_time::Timer::after_secs(2).await;

        // Fill with green
        info!("Filling screen with GREEN...");
        display.fill_color(GREEN);
        display.flush().await.unwrap();
        embassy_time::Timer::after_secs(2).await;

        // Fill with blue
        info!("Filling screen with BLUE...");
        display.fill_color(BLUE);
        display.flush().await.unwrap();
        embassy_time::Timer::after_secs(2).await;

        // Pattern 1: Complex Checkerboard with Multiple Colors
        info!("Pattern 1: Complex Multi-Color Checkerboard (90°)");

        // Clear frame buffer first
        display.clear_frame_buffer(BLACK);

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

                // Fill the block area in frame buffer
                display.fill_rect(x, y, block_width, block_height, color);
            }
        }

        // Flush the frame buffer to display
        display.flush().await.unwrap();

        info!("Complex checkerboard pattern completed");
        embassy_time::Timer::after_secs(5).await;

        // Pattern 2: 10x10 Color Checkerboard
        info!("Pattern 2: 10x10 Color Checkerboard (90°)");

        // Clear frame buffer first
        display.clear_frame_buffer(BLACK);

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

                // Fill the block area in frame buffer
                display.fill_rect(x, y, block_width, block_height, color);
            }
        }

        // Flush the frame buffer to display
        display.flush().await.unwrap();

        info!("Pattern 2 completed");
        embassy_time::Timer::after_secs(5).await;

        // Pattern 3: Gradient Stripes
        info!("Pattern 3: Gradient Color Stripes (90°)");

        // Clear frame buffer first
        display.clear_frame_buffer(BLACK);

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

                // Fill one horizontal line of the stripe in frame buffer
                display.fill_rect(x, y, stripe_width, 1, gradient_color);
            }
        }

        // Flush the frame buffer to display
        display.flush().await.unwrap();

        info!("Gradient stripes pattern completed");
        embassy_time::Timer::after_secs(5).await;

        // Pattern 4: Concentric Rectangles
        info!("Pattern 4: Concentric Rectangles (90°)");

        // Clear frame buffer first
        display.clear_frame_buffer(BLACK);

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
                        display.fill_rect(start_x as u16, border_y as u16, width as u16, 1, color);
                    }
                }
            }

            // Left and right borders for 160×40 logical screen
            for border_x in [layer * 16, 159 - layer * 16] {
                if border_x < 160 {
                    let start_y = layer * 4;
                    let end_y = 39 - layer * 4;
                    if start_y <= end_y && end_y < 40 {
                        let height = end_y - start_y + 1;
                        display.fill_rect(border_x as u16, start_y as u16, 1, height as u16, color);
                    }
                }
            }
        }

        // Flush the frame buffer to display
        display.flush().await.unwrap();

        info!("Concentric rectangles pattern completed");
        embassy_time::Timer::after_secs(5).await;

        // Pattern 5: Diagonal Lines Pattern
        info!("Pattern 5: Diagonal Lines Pattern (90°)");

        // Clear frame buffer first
        display.clear_frame_buffer(BLACK);

        // Draw diagonal lines across the screen for 160×40 logical screen
        for line in 0..20 {
            let color = colors[line % colors.len()];
            let spacing = 8;

            // Draw diagonal line from top-left to bottom-right (160×40)
            for step in 0..200 {
                let x = (step + line * spacing) % 160; // screen width
                let y = (step * 40 / 160) % 40;        // screen height

                if x < 160 && y < 40 {
                    display.set_pixel(x as u16, y as u16, color);
                }
            }
        }

        // Flush the frame buffer to display
        display.flush().await.unwrap();

        info!("Diagonal lines pattern completed");
        embassy_time::Timer::after_secs(5).await;

        // Pattern 6: Rainbow Gradient with Saturation
        info!("Pattern 6: Rainbow Gradient with Saturation (90°)");

        // Clear frame buffer first
        display.clear_frame_buffer(BLACK);

        // Create rainbow gradient: long edge (160px) = hue, short edge (40px) = saturation
        for y in 0..LOGICAL_HEIGHT {
            for x in 0..LOGICAL_WIDTH {
                // Hue varies along the long edge (160 pixels)
                let hue = (x as f32 / LOGICAL_WIDTH as f32) * 360.0;

                // Saturation varies along the short edge (40 pixels): 0% to 100%
                let saturation = y as f32 / (LOGICAL_HEIGHT - 1) as f32;

                // Convert HSV to RGB565
                let rgb = hsv_to_rgb565(hue, saturation, 1.0); // Full brightness

                display.set_pixel(x as u16, y as u16, rgb);
            }
        }

        // Flush the frame buffer to display
        display.flush().await.unwrap();

        info!("Rainbow gradient pattern completed");
        embassy_time::Timer::after_secs(5).await;

        // Pattern 7: 20×20 Triangles Pattern
        info!("Pattern 7: 20×20 Triangles Pattern (90°)");

        // Clear frame buffer first
        display.clear_frame_buffer(BLACK);

        // Draw triangles in a grid pattern for 160×40 logical screen
        // Each triangle is 20×20 pixels, so we can fit 8×2 triangles
        let triangle_size = 20;
        let triangles_x = LOGICAL_WIDTH / triangle_size;  // 8 triangles across
        let triangles_y = LOGICAL_HEIGHT / triangle_size; // 2 triangles down

        for row in 0..triangles_y {
            for col in 0..triangles_x {
                let color = colors[(row * triangles_x + col) % colors.len()];
                let base_x = col * triangle_size;
                let base_y = row * triangle_size;

                // Draw all triangles pointing upward
                draw_triangle_up(&mut display, base_x, base_y, triangle_size, color);
            }
        }

        // Flush the frame buffer to display
        display.flush().await.unwrap();

        info!("20×20 triangles pattern completed");
        embassy_time::Timer::after_secs(5).await;
    }
}

// Helper functions to draw triangles in different orientations
fn draw_triangle_up<SPI, DC, RST, TIMER, BusE, PinE>(
    display: &mut GC9D01<SPI, DC, RST, TIMER>,
    base_x: usize,
    base_y: usize,
    size: usize,
    color: Rgb565,
) where
    SPI: embedded_hal_async::spi::SpiDevice<Error = BusE>,
    DC: embedded_hal::digital::OutputPin<Error = PinE>,
    RST: embedded_hal::digital::OutputPin<Error = PinE>,
    TIMER: Gc9d01Timer,
    BusE: core::fmt::Debug + embedded_hal::spi::Error,
    PinE: core::fmt::Debug,
{
    // Draw upward pointing isosceles triangle
    // Bottom edge is horizontal with length = size (20 pixels)
    // Triangle height = size (20 pixels)
    // y=0 is at the top (apex), y=size-1 is at the bottom (base)

    for y in 0..size {
        // Calculate width at this height level
        // At y=0 (top): width = 1 pixel
        // At y=size-1 (bottom): width = size pixels
        let width = if y == 0 {
            1
        } else {
            (y * size) / (size - 1)
        };

        // Center the line horizontally within the size x size area
        let start_x = base_x + (size - width) / 2;
        let end_x = start_x + width;

        for x in start_x..end_x.min(base_x + size) {
            if x < LOGICAL_WIDTH && (base_y + y) < LOGICAL_HEIGHT {
                display.set_pixel(x as u16, (base_y + y) as u16, color);
            }
        }
    }
}

fn draw_triangle_down<SPI, DC, RST, TIMER, BusE, PinE>(
    display: &mut GC9D01<SPI, DC, RST, TIMER>,
    base_x: usize,
    base_y: usize,
    size: usize,
    color: Rgb565,
) where
    SPI: embedded_hal_async::spi::SpiDevice<Error = BusE>,
    DC: embedded_hal::digital::OutputPin<Error = PinE>,
    RST: embedded_hal::digital::OutputPin<Error = PinE>,
    TIMER: Gc9d01Timer,
    BusE: core::fmt::Debug + embedded_hal::spi::Error,
    PinE: core::fmt::Debug,
{
    // Draw downward pointing triangle
    for y in 0..size {
        let width = ((size - y - 1) * 2) + 1;
        let start_x = base_x + y;
        let end_x = start_x + width;

        for x in start_x..end_x.min(base_x + size) {
            if x < LOGICAL_WIDTH && (base_y + y) < LOGICAL_HEIGHT {
                display.set_pixel(x as u16, (base_y + y) as u16, color);
            }
        }
    }
}

fn draw_triangle_left<SPI, DC, RST, TIMER, BusE, PinE>(
    display: &mut GC9D01<SPI, DC, RST, TIMER>,
    base_x: usize,
    base_y: usize,
    size: usize,
    color: Rgb565,
) where
    SPI: embedded_hal_async::spi::SpiDevice<Error = BusE>,
    DC: embedded_hal::digital::OutputPin<Error = PinE>,
    RST: embedded_hal::digital::OutputPin<Error = PinE>,
    TIMER: Gc9d01Timer,
    BusE: core::fmt::Debug + embedded_hal::spi::Error,
    PinE: core::fmt::Debug,
{
    // Draw left pointing triangle
    for x in 0..size {
        let height = (x * 2) + 1;
        let start_y = base_y + (size - x - 1);
        let end_y = start_y + height;

        for y in start_y..end_y.min(base_y + size) {
            if (base_x + x) < LOGICAL_WIDTH && y < LOGICAL_HEIGHT {
                display.set_pixel((base_x + x) as u16, y as u16, color);
            }
        }
    }
}

fn draw_triangle_right<SPI, DC, RST, TIMER, BusE, PinE>(
    display: &mut GC9D01<SPI, DC, RST, TIMER>,
    base_x: usize,
    base_y: usize,
    size: usize,
    color: Rgb565,
) where
    SPI: embedded_hal_async::spi::SpiDevice<Error = BusE>,
    DC: embedded_hal::digital::OutputPin<Error = PinE>,
    RST: embedded_hal::digital::OutputPin<Error = PinE>,
    TIMER: Gc9d01Timer,
    BusE: core::fmt::Debug + embedded_hal::spi::Error,
    PinE: core::fmt::Debug,
{
    // Draw right pointing triangle
    for x in 0..size {
        let height = ((size - x - 1) * 2) + 1;
        let start_y = base_y + x;
        let end_y = start_y + height;

        for y in start_y..end_y.min(base_y + size) {
            if (base_x + x) < LOGICAL_WIDTH && y < LOGICAL_HEIGHT {
                display.set_pixel((base_x + x) as u16, y as u16, color);
            }
        }
    }
}