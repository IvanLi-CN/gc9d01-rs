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
use embassy_time::Timer;
use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, Line, Rectangle, Triangle, PrimitiveStyle},
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    text::{Baseline, Text},
};
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
        Timer::after_millis(milliseconds).await;
    }
}

// Color definitions
const RED: Rgb565 = Rgb565::RED;
const GREEN: Rgb565 = Rgb565::GREEN;
const BLUE: Rgb565 = Rgb565::BLUE;
const YELLOW: Rgb565 = Rgb565::YELLOW;
const MAGENTA: Rgb565 = Rgb565::MAGENTA;
const CYAN: Rgb565 = Rgb565::CYAN;
const WHITE: Rgb565 = Rgb565::WHITE;
const BLACK: Rgb565 = Rgb565::BLACK;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Starting GC9D01 Embedded Graphics Demo");

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

    // SPI configuration
    let mut spi_config = SpiConfig::default();
    spi_config.frequency = Hertz(16_000_000); // 16 MHz

    let spi = Stm32Spi::new_txonly(
        p.SPI1,
        p.PB3, // SCK
        p.PA7, // MOSI
        p.DMA1_CH3, // TX DMA
        spi_config,
    );

    // Create shared SPI bus
    static SPI_BUS: StaticCell<Mutex<CriticalSectionRawMutex, Stm32Spi<'static, mode::Async>>> = StaticCell::new();
    let spi_bus = SPI_BUS.init(Mutex::new(spi));

    // Create CS pin for display
    let cs_pin = Output::new(p.PA4, Level::High, Speed::VeryHigh);
    let spi_device = EmbassySpiDevice::new(spi_bus, cs_pin);

    // Create control pins
    let dc_pin = Output::new(p.PB0, Level::Low, Speed::VeryHigh);
    let rst_pin = Output::new(p.PC4, Level::High, Speed::VeryHigh);

    // Display configuration for 90° orientation (160×40 logical)
    // Use Portrait orientation to match the working reference example
    let display_config = DisplayDriverConfig {
        width: 160,
        height: 40,
        orientation: Orientation::Portrait,
        rgb: false,
        inverted: false,
        dx: 0,
        dy: 0,
    };

    static DISPLAY_BUFFER_CELL: StaticCell<[u8; gc9d01::BUF_SIZE]> = StaticCell::new();
    let buffer_slice: &mut [u8] = DISPLAY_BUFFER_CELL.init([0; gc9d01::BUF_SIZE]);

    // Create frame buffer for embedded-graphics
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
    > = GC9D01::new(display_config, spi_device, dc_pin, rst_pin, buffer_slice, frame_buffer);

    info!("Initializing display...");
    match display.init().await {
        Ok(_) => info!("Display initialized successfully!"),
        Err(e) => error!("Display initialization failed: {:?}", e),
    }

    info!("Starting embedded-graphics demonstrations matching reference code...");

    // Clear entire screen using black first
    info!("Clearing entire logical screen area (160x40) with black...");
    display.clear(BLACK).unwrap();
    display.flush().await.unwrap();
    info!("Entire logical screen area cleared with black");
    Timer::after_secs(2).await;

    loop {
        // Test 0: Basic fill test to verify display is working (matching reference)
        info!("Test 0: Basic Fill Test");

        // Fill with red
        info!("Filling screen with RED...");
        display.clear(RED).unwrap();
        display.flush().await.unwrap();
        Timer::after_secs(2).await;

        // Fill with green
        info!("Filling screen with GREEN...");
        display.clear(GREEN).unwrap();
        display.flush().await.unwrap();
        Timer::after_secs(2).await;

        // Fill with blue
        info!("Filling screen with BLUE...");
        display.clear(BLUE).unwrap();
        display.flush().await.unwrap();
        Timer::after_secs(2).await;

        // Pattern 1: Complex Checkerboard with Multiple Colors (matching reference)
        info!("Pattern 1: Complex Multi-Color Checkerboard (90°)");
        display.clear(BLACK).unwrap();

        // Create a complex checkerboard pattern for 160×40 logical screen
        let block_width = 20;  // 160 / 8 = 20 pixels wide
        let block_height = 20; // 40 / 2 = 20 pixels high
        let blocks_x = 8;      // 8 blocks across (160 pixels width)
        let blocks_y = 2;      // 2 blocks down (40 pixels height)

        let colors = [RED, GREEN, BLUE, YELLOW, MAGENTA, CYAN, WHITE, BLACK];

        for row in 0..blocks_y {
            for col in 0..blocks_x {
                let color_index = ((row * blocks_x + col) as usize) % colors.len();
                let color = colors[color_index];

                let x = col * block_width;
                let y = row * block_height;

                // Fill the block area
                Rectangle::new(Point::new(x as i32, y as i32), Size::new(block_width as u32, block_height as u32))
                    .into_styled(PrimitiveStyle::with_fill(color))
                    .draw(&mut display).unwrap();
            }
        }

        display.flush().await.unwrap();
        info!("Complex checkerboard pattern completed");
        Timer::after_secs(5).await;

        // Pattern 2: 10x10 Color Checkerboard (matching reference)
        info!("Pattern 2: 10x10 Color Checkerboard (90°)");
        display.clear(BLACK).unwrap();

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

                // Fill the block area
                Rectangle::new(Point::new(x as i32, y as i32), Size::new(block_width as u32, block_height as u32))
                    .into_styled(PrimitiveStyle::with_fill(color))
                    .draw(&mut display).unwrap();
            }
        }

        display.flush().await.unwrap();
        info!("Pattern 2 completed");
        Timer::after_secs(5).await;

        // Now test the original embedded-graphics demonstrations
        info!("=== Starting Original Embedded-Graphics Demonstrations ===");

        // Demo 1: Basic shapes
        info!("Demo 1: Basic Shapes");
        display.clear(BLACK).unwrap();

        // Draw rectangles
        Rectangle::new(Point::new(10, 5), Size::new(30, 15))
            .into_styled(PrimitiveStyle::with_stroke(RED, 1))
            .draw(&mut display).unwrap();

        Rectangle::new(Point::new(50, 8), Size::new(25, 10))
            .into_styled(PrimitiveStyle::with_fill(GREEN))
            .draw(&mut display).unwrap();

        // Draw circles
        Circle::new(Point::new(90, 10), 15)
            .into_styled(PrimitiveStyle::with_stroke(BLUE, 2))
            .draw(&mut display).unwrap();

        Circle::new(Point::new(120, 15), 8)
            .into_styled(PrimitiveStyle::with_fill(YELLOW))
            .draw(&mut display).unwrap();

        display.flush().await.unwrap();
        Timer::after_secs(3).await;

        // Demo 2: Lines and patterns
        info!("Demo 2: Lines and Patterns");
        display.clear(BLACK).unwrap();

        // Draw diagonal lines
        for i in 0..8 {
            let color = match i % 3 {
                0 => RED,
                1 => GREEN,
                _ => BLUE,
            };
            Line::new(Point::new(i * 20, 0), Point::new(i * 20 + 20, 39))
                .into_styled(PrimitiveStyle::with_stroke(color, 1))
                .draw(&mut display).unwrap();
        }

        display.flush().await.unwrap();
        Timer::after_secs(3).await;

        // Demo 3: Text rendering
        info!("Demo 3: Text Rendering");
        display.clear(BLACK).unwrap();

        let text_style = MonoTextStyle::new(&FONT_6X10, WHITE);
        Text::with_baseline("GC9D01", Point::new(10, 15), text_style, Baseline::Top)
            .draw(&mut display).unwrap();

        Text::with_baseline("Graphics", Point::new(10, 30), text_style, Baseline::Top)
            .draw(&mut display).unwrap();

        display.flush().await.unwrap();
        Timer::after_secs(3).await;

        // Demo 4: Triangles
        info!("Demo 4: Triangles");
        display.clear(BLACK).unwrap();

        Triangle::new(Point::new(20, 5), Point::new(10, 25), Point::new(30, 25))
            .into_styled(PrimitiveStyle::with_fill(MAGENTA))
            .draw(&mut display).unwrap();

        Triangle::new(Point::new(60, 10), Point::new(50, 30), Point::new(70, 30))
            .into_styled(PrimitiveStyle::with_stroke(CYAN, 2))
            .draw(&mut display).unwrap();

        display.flush().await.unwrap();
        Timer::after_secs(3).await;

        // Demo 5: Complex pattern
        info!("Demo 5: Complex Pattern");
        display.clear(BLACK).unwrap();

        // Create a grid pattern
        for x in (0..160).step_by(20) {
            for y in (0..40).step_by(10) {
                let color = if (x / 20 + y / 10) % 2 == 0 { WHITE } else { BLACK };
                Rectangle::new(Point::new(x, y), Size::new(20, 10))
                    .into_styled(PrimitiveStyle::with_fill(color))
                    .draw(&mut display).unwrap();
            }
        }

        display.flush().await.unwrap();
        Timer::after_secs(3).await;
    }
}
