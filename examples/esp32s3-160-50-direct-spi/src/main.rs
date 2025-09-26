#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::Timer;

use esp_backtrace as _; // panic handler + backtrace
use esp_println as _;   // defmt bridge for espflash

use defmt::*;

use esp_hal::gpio::{Level, Output};
use esp_hal::spi::master::{Config as SpiConfig, Spi};
use esp_hal::spi::Mode as SpiMode;
use esp_hal::timer::timg::TimerGroup;

use embedded_hal_async::spi::SpiBus; // for .write().await

// Screen orientation enumeration for GC9D01 (MADCTL bits)
#[derive(Clone, Copy, Debug)]
pub enum ScreenOrientation {
    Portrait = 0x00,           // 0°
    Landscape = 0xA0,          // 90° (MY|MV)
    PortraitSwapped = 0x60,    // 180° (MX|MY)
    LandscapeSwapped = 0x20,   // 270° (MV)
}

// Active logical size: 160x50 (W x H)
const DISPLAY_WIDTH: u16 = 160;  // columns
const DISPLAY_HEIGHT: u16 = 50;  // rows

// GC9D01 initialization function based on vendor sample
async fn initialize_gc9d01(
    spi: &mut impl SpiBus,
    cs_pin: &mut Output<'_>,
    dc_pin: &mut Output<'_>,
    rst_pin: &mut Output<'_>,
    orientation: ScreenOrientation,
) {
    // keep parameter used to silence warnings if orientation is not used here
    let _ = orientation;
    // Helper function to send command with multiple data bytes
    async fn write_command_with_data(
        spi: &mut impl SpiBus,
        cs: &mut Output<'_>,
        dc: &mut Output<'_>,
        cmd: u8,
        data: &[u8],
    ) {
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

    async fn write_command(spi: &mut impl SpiBus, cs: &mut Output<'_>, dc: &mut Output<'_>, cmd: u8) {
        dc.set_low();  // Command mode
        cs.set_low();  // Select device
        let _ = spi.write(&[cmd]).await;
        cs.set_high(); // Deselect device
    }

    // GC9D01 hardware reset sequence (exactly as vendor sample)
    info!("Performing GC9D01 hardware reset (vendor sequence)...");
    rst_pin.set_high();
    Timer::after_millis(50).await;
    rst_pin.set_low();
    Timer::after_millis(50).await;
    rst_pin.set_high();
    Timer::after_millis(120).await; // Wait for display to stabilize (120ms)
    info!("GC9D01 hardware reset completed");

    info!("Starting GC9D01 initialization sequence (vendor sample)...");

    // Enable internal register access (0xFE, 0xEF)
    write_command(spi, cs_pin, dc_pin, 0xFE).await;
    write_command(spi, cs_pin, dc_pin, 0xEF).await;

    // Internal register block
    write_command_with_data(spi, cs_pin, dc_pin, 0x86, &[0xFF]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0x87, &[0xFF]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0x8E, &[0xFF]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0x8F, &[0xFF]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0x80, &[0x13]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0x81, &[0x40]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0x82, &[0x0A]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0x83, &[0x0B]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0x84, &[0x60]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0x85, &[0x80]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0x89, &[0x10]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0x8A, &[0x0F]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0x8B, &[0x02]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0x8C, &[0x59]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0x8D, &[0x55]).await;

    // Pixel format RGB565
    write_command_with_data(spi, cs_pin, dc_pin, 0x3A, &[0x05]).await;

    // Rotation/scan related
    write_command_with_data(spi, cs_pin, dc_pin, 0xEC, &[0x00]).await;

    // VGL size
    write_command_with_data(spi, cs_pin, dc_pin, 0x7E, &[0x30]).await;

    // Frame frequency
    write_command_with_data(spi, cs_pin, dc_pin, 0x74, &[0x05, 0x4D, 0x00, 0x00, 0x01, 0x00, 0x00]).await;

    // Porch
    write_command_with_data(spi, cs_pin, dc_pin, 0xB5, &[0x0D, 0x0D]).await;

    // Scan direction (forward: 0x00, backward: 0x60)
    write_command_with_data(spi, cs_pin, dc_pin, 0xB6, &[0x00, 0x00]).await;

    // GIP timing
    write_command_with_data(spi, cs_pin, dc_pin, 0x60, &[0x38, 0x09, 0x1E, 0x7A]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0x63, &[0x38, 0xAE, 0x1E, 0x7A]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0x64, &[0x38, 0x0B, 0x70, 0xAB, 0x1E, 0x7A]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0x66, &[0x38, 0x0F, 0x70, 0xAF, 0x1E, 0x7A]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0x68, &[0x00, 0x08, 0x07, 0x00, 0x07, 0x55, 0x6A]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0x6A, &[0x00, 0x00]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0x6C, &[0x22, 0x02, 0x22, 0x02, 0x22, 0x22, 0x50]).await;
    write_command_with_data(
        spi,
        cs_pin,
        dc_pin,
        0x6E,
        &[
            0x00, 0x00, 0x00, 0x02, 0x14, 0x12, 0x0C, 0x0A, 0x1E, 0x1D, 0x08, 0x00, 0x16, 0x15, 0x00, 0x00,
            0x00, 0x00, 0x15, 0x16, 0x00, 0x07, 0x1D, 0x1E, 0x09, 0x0B, 0x11, 0x13, 0x01, 0x00, 0x00, 0x00,
        ],
    )
    .await;

    // Internal voltage adjustments
    write_command_with_data(spi, cs_pin, dc_pin, 0x98, &[0x3E]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0x99, &[0x3E]).await;

    write_command_with_data(spi, cs_pin, dc_pin, 0x9B, &[0x3B]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0x93, &[0x33, 0x7F, 0x00]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0x91, &[0x0E, 0x09]).await;

    // VGH/VGL CLK
    write_command_with_data(spi, cs_pin, dc_pin, 0x70, &[0x04, 0x02, 0x0D, 0x04, 0x02, 0x0D]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0x71, &[0x04, 0x02, 0x0D]).await;

    // VREG voltage
    write_command_with_data(spi, cs_pin, dc_pin, 0xC3, &[0x26]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0xC4, &[0x26]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0xC9, &[0x1C]).await;

    // Gamma
    write_command_with_data(spi, cs_pin, dc_pin, 0xF0, &[0x02, 0x03, 0x0A, 0x06, 0x00, 0x1A]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0xF2, &[0x02, 0x03, 0x0A, 0x06, 0x00, 0x1A]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0xF1, &[0x38, 0x78, 0x1B, 0x2E, 0x2F, 0xC8]).await;
    write_command_with_data(spi, cs_pin, dc_pin, 0xF3, &[0x38, 0x74, 0x12, 0x2E, 0x2F, 0xDF]).await;

    // Single gate mode
    write_command_with_data(spi, cs_pin, dc_pin, 0xBF, &[0x00]).await;

    // SOU related
    write_command_with_data(spi, cs_pin, dc_pin, 0xF9, &[0x40]).await;

    // Memory access control (MADCTL) fixed to 0x00 (vendor sample)
    write_command_with_data(spi, cs_pin, dc_pin, 0x36, &[0x00]).await;

    // Set initial window as vendor sample
    write_command(spi, cs_pin, dc_pin, 0x2A).await;
    write_data_slice(spi, cs_pin, dc_pin, &[0x00, 0x0F, 0x00, 0x40]).await;
    write_command(spi, cs_pin, dc_pin, 0x2B).await;
    write_data_slice(spi, cs_pin, dc_pin, &[0x00, 0x00, 0x00, 0x9F]).await;

    // Sleep out -> Display on
    write_command(spi, cs_pin, dc_pin, 0x11).await;
    Timer::after_millis(200).await;
    write_command(spi, cs_pin, dc_pin, 0x29).await;
    write_command(spi, cs_pin, dc_pin, 0x2C).await;

    info!("GC9D01 initialization complete (vendor sample)");
}

// SPI helpers
async fn write_command(spi: &mut impl SpiBus, cs: &mut Output<'_>, dc: &mut Output<'_>, cmd: u8) {
    dc.set_low();
    cs.set_low();
    let _ = spi.write(&[cmd]).await;
    cs.set_high();
}

async fn write_data_slice(spi: &mut impl SpiBus, cs: &mut Output<'_>, dc: &mut Output<'_>, data: &[u8]) {
    dc.set_high();
    cs.set_low();
    let _ = spi.write(data).await;
    cs.set_high();
}

// Optional panel offsets for modules with shifted active area
const X_OFFSET: u16 = 15; // vendor sample sets column from 0x000F
const Y_OFFSET: u16 = 0;

async fn set_address_window(
    spi: &mut impl SpiBus,
    cs: &mut Output<'_>,
    dc: &mut Output<'_>,
    x0: u16,
    y0: u16,
    x1: u16,
    y1: u16,
) {
    let x0 = x0 + X_OFFSET;
    let x1 = x1 + X_OFFSET;
    let y0 = y0 + Y_OFFSET;
    let y1 = y1 + Y_OFFSET;
    // Column address set (0x2A)
    write_command(spi, cs, dc, 0x2A).await;
    write_data_slice(
        spi,
        cs,
        dc,
        &[(x0 >> 8) as u8, (x0 & 0xFF) as u8, (x1 >> 8) as u8, (x1 & 0xFF) as u8],
    )
    .await;

    // Row address set (0x2B)
    write_command(spi, cs, dc, 0x2B).await;
    write_data_slice(
        spi,
        cs,
        dc,
        &[(y0 >> 8) as u8, (y0 & 0xFF) as u8, (y1 >> 8) as u8, (y1 & 0xFF) as u8],
    )
    .await;
}

async fn start_memory_write(spi: &mut impl SpiBus, cs: &mut Output<'_>, dc: &mut Output<'_>) {
    write_command(spi, cs, dc, 0x2C).await;
}

// Fill area with solid color - optimized batch version with orientation support
async fn fill_area_with_color(
    spi: &mut impl SpiBus,
    cs: &mut Output<'_>,
    dc: &mut Output<'_>,
    x0: u16,
    y0: u16,
    x1: u16,
    y1: u16,
    color: u16,
    orientation: ScreenOrientation,
) {
    let width = x1 - x0 + 1;
    let height = y1 - y0 + 1;
    let pixel_count = (width as u32) * (height as u32);

    // Normalize inputs
    let (mut nx0, mut nx1) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
    let (mut ny0, mut ny1) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };

    // Apply rotation transform (assumes MADCTL = 0x00 during init)
    let (final_x0, final_y0, final_x1, final_y1) = match orientation {
        // 0°
        ScreenOrientation::Portrait => (nx0, ny0, nx1, ny1),
        // 90° clockwise: (x, y) -> (y, W-1-x)
        ScreenOrientation::Landscape => (
            ny0,
            DISPLAY_WIDTH - 1 - nx1,
            ny1,
            DISPLAY_WIDTH - 1 - nx0,
        ),
        // 180°: (x, y) -> (W-1-x, H-1-y)
        ScreenOrientation::PortraitSwapped => (
            DISPLAY_WIDTH - 1 - nx1,
            DISPLAY_HEIGHT - 1 - ny1,
            DISPLAY_WIDTH - 1 - nx0,
            DISPLAY_HEIGHT - 1 - ny0,
        ),
        // 270° clockwise: (x, y) -> (H-1-y, x)
        ScreenOrientation::LandscapeSwapped => (
            DISPLAY_HEIGHT - 1 - ny1,
            nx0,
            DISPLAY_HEIGHT - 1 - ny0,
            nx1,
        ),
    };

    set_address_window(spi, cs, dc, final_x0, final_y0, final_x1, final_y1).await;
    start_memory_write(spi, cs, dc).await;

    let color_bytes = [(color >> 8) as u8, (color & 0xFF) as u8];

    const BATCH_SIZE: usize = 512; // 256 pixels per batch
    let mut batch_buffer = [0u8; BATCH_SIZE];
    for i in (0..BATCH_SIZE).step_by(2) {
        batch_buffer[i] = color_bytes[0];
        batch_buffer[i + 1] = color_bytes[1];
    }

    let pixels_per_batch = BATCH_SIZE / 2;
    let full_batches = pixel_count as usize / pixels_per_batch;
    let remaining_pixels = pixel_count as usize % pixels_per_batch;

    for batch in 0..full_batches {
        let _ = write_data_slice(spi, cs, dc, &batch_buffer).await;
    }

    if remaining_pixels > 0 {
        let remaining_bytes = remaining_pixels * 2;
        let _ = write_data_slice(spi, cs, dc, &batch_buffer[0..remaining_bytes]).await;
    }
}

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_hal_embassy::main]
async fn main(_spawner: Spawner) {
    let p = esp_hal::init(esp_hal::Config::default());
    info!("Starting GC9D01 Direct SPI Test Firmware (ESP32-S3)");

    // Initialize the embassy time driver
    let timg0 = TimerGroup::new(p.TIMG0);
    esp_hal_embassy::init(timg0.timer0);
    info!("init.time: embassy-timer=ok");

    // SPI and GPIO initialization
    info!("Initializing SPI and GPIO pins...");

    // Pins per iso-usb-hub_v2 project
    let sclk = p.GPIO12; // SCK
    let mosi = p.GPIO11; // MOSI

    let spi = Spi::new(
        p.SPI2,
        SpiConfig::default()
            .with_frequency(esp_hal::time::Rate::from_hz(10_000_000))
            .with_mode(SpiMode::_0),
    )
    .expect("spi.init")
    .with_sck(sclk)
    .with_mosi(mosi)
    .into_async();
    let mut spi = spi; // mutable for trait methods

    let mut cs_pin = Output::new(p.GPIO13, Level::High, esp_hal::gpio::OutputConfig::default());
    let mut dc_pin = Output::new(p.GPIO10, Level::Low, esp_hal::gpio::OutputConfig::default());
    let mut rst_pin = Output::new(p.GPIO14, Level::High, esp_hal::gpio::OutputConfig::default());
    let mut blk_pin = Output::new(p.GPIO15, Level::Low, esp_hal::gpio::OutputConfig::default());
    blk_pin.set_high(); // Enable backlight

    info!("SPI and GPIO initialized successfully");

    // Choose display orientation here
    const ORIENTATION: ScreenOrientation = ScreenOrientation::Landscape; // not used by init (MADCTL fixed), kept for clarity
    const DRAW_ORIENTATION: ScreenOrientation = ScreenOrientation::Landscape; // rotate logical 160x50 -> physical 50x160

    // Initialize GC9D01 with selected orientation
    initialize_gc9d01(&mut spi, &mut cs_pin, &mut dc_pin, &mut rst_pin, ORIENTATION).await;

    // Rendering test: two rows (8 colors + 8 grayscale), 20 px tall each
    info!("Starting display test - 8 colors + 8 grayscale rows");

    // RGB565 colors
    const MAGENTA: u16 = 0xF81F;
    const RED: u16 = 0xF800;
    const YELLOW: u16 = 0xFFE0;
    const GREEN: u16 = 0x07E0;
    const CYAN: u16 = 0x07FF;
    const BLUE: u16 = 0x001F;
    const PURPLE: u16 = 0x781F; // Dark magenta-ish
    const WHITE: u16 = 0xFFFF;
    const BLACK: u16 = 0x0000;

    // First clear the logical 160x50 to black to avoid ghosting
    info!("Clearing logical area (160x50) with black...");
    fill_area_with_color(
        &mut spi,
        &mut cs_pin,
        &mut dc_pin,
        0,  // x0
        0,  // y0
        DISPLAY_WIDTH - 1,
        DISPLAY_HEIGHT - 1,
        BLACK,
        DRAW_ORIENTATION,
    )
    .await;
    info!("Logical 160x50 area cleared with black");
    Timer::after_millis(300).await;

    // Draw top row: 8 colors, each 20x20
    let colors = [MAGENTA, RED, YELLOW, GREEN, CYAN, BLUE, PURPLE, WHITE];
    info!("Drawing top row: 8 colors (20x20 each)");
    for (i, &color) in colors.iter().enumerate() {
        let x_start = (i as u16) * 20; // 8 blocks across in width=160 -> 20px each
        let x_end = x_start + 19;
        let y_start = 0;
        let y_end = 19;
        fill_area_with_color(
            &mut spi,
            &mut cs_pin,
            &mut dc_pin,
            x_start,
            y_start,
            x_end,
            y_end,
            color,
            DRAW_ORIENTATION,
        )
        .await;
    }

    // Draw bottom row: 8 grayscale levels, each 20x20
    let grayscale = [0x0000, 0x2104, 0x4208, 0x630C, 0x8410, 0xA514, 0xC618, 0xFFFF];
    info!("Drawing bottom row: 8 grayscale levels (20x20 each)");
    for (i, &gray_color) in grayscale.iter().enumerate() {
        let x_start = (i as u16) * 20;
        let x_end = x_start + 19;
        let y_start = 20;
        let y_end = 39;
        fill_area_with_color(
            &mut spi,
            &mut cs_pin,
            &mut dc_pin,
            x_start,
            y_start,
            x_end,
            y_end,
            gray_color,
            DRAW_ORIENTATION,
        )
        .await;
    }

    info!("Rendering tests completed. Example will idle.");

    loop {
        Timer::after_millis(1000).await;
    }
}
