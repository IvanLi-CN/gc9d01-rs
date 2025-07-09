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



    // Fill area with solid color - optimized batch version
    async fn fill_area_with_color(
        spi: &mut Spi<'_, Async>,
        cs: &mut Output<'_>,
        dc: &mut Output<'_>,
        x0: u16, y0: u16, x1: u16, y1: u16,
        color: u16
    ) {
        let width = x1 - x0 + 1;
        let height = y1 - y0 + 1;
        let pixel_count = (width as u32) * (height as u32);

        debug!("Filling area {}x{} ({} pixels) with color 0x{:04X}", width, height, pixel_count, color);

        set_address_window(spi, cs, dc, x0, y0, x1, y1).await;
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

    // GC9D01 hardware reset sequence
    info!("Performing GC9D01 hardware reset...");
    rst_pin.set_low();   // Assert reset (active low)
    Timer::after_millis(10).await;  // Hold reset for 10ms
    rst_pin.set_high();  // Release reset
    Timer::after_millis(120).await; // Wait for display to stabilize (120ms)
    info!("GC9D01 hardware reset completed");

    // GC9D01 complete initialization sequence based on reference document
    info!("Starting GC9D01 complete initialization sequence...");

    // Enable internal register access (0xFE, 0xEF)
    write_command(&mut spi, &mut cs_pin, &mut dc_pin, 0xFE).await;
    write_command(&mut spi, &mut cs_pin, &mut dc_pin, 0xEF).await;
    info!("Internal register access enabled");

    // Internal register settings 0x80-0x8F (all set to 0xFF)
    info!("Setting internal registers 0x80-0x8F...");
    for reg in 0x80..=0x8F {
        write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, reg, &[0xFF]).await;
    }

    // Pixel format setting - RGB565 (16-bit color)
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0x3A, &[0x05]).await;
    info!("Pixel format set to RGB565");

    // Display rotation setting
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0xEC, &[0x11]).await;
    info!("Display rotation configured");

    // VGL size setting
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0x7E, &[0x7A]).await;
    info!("VGL size configured");

    // Frame frequency modification
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0x74, &[
        0x02, 0x0E, 0x00, 0x00, 0x28, 0x00, 0x00
    ]).await;
    info!("Frame frequency configured");

    // Internal voltage adjustment
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0x98, &[0x3E]).await;
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0x99, &[0x3E]).await;
    info!("Internal voltage adjusted");

    // Internal porch settings
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0xB5, &[0x0E, 0x0E]).await;
    info!("Internal porch configured");

    // GIP timing start
    info!("Configuring GIP timing...");
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0x60, &[
        0x38, 0x09, 0x6D, 0x67
    ]).await;

    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0x63, &[
        0x38, 0xAD, 0x6D, 0x67, 0x05
    ]).await;

    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0x64, &[
        0x38, 0x0B, 0x70, 0xAB, 0x6D, 0x67
    ]).await;

    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0x66, &[
        0x38, 0x0F, 0x70, 0xAF, 0x6D, 0x67
    ]).await;

    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0x6A, &[0x00, 0x00]).await;

    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0x68, &[
        0x3B, 0x08, 0x04, 0x00, 0x04, 0x64, 0x67
    ]).await;

    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0x6C, &[
        0x22, 0x02, 0x22, 0x02, 0x22, 0x22, 0x50
    ]).await;

    // Complex GIP timing data for 0x6E command
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0x6E, &[
        0x00, 0x00, 0x00, 0x00, 0x07, 0x01, 0x13, 0x11,
        0x0B, 0x09, 0x16, 0x15, 0x1D, 0x1E, 0x00, 0x00,
        0x00, 0x00, 0x1E, 0x1D, 0x15, 0x16, 0x0A, 0x0C,
        0x12, 0x14, 0x02, 0x08, 0x00, 0x00, 0x00, 0x00
    ]).await;
    info!("GIP timing configured");

    // Internal voltage settings start
    info!("Configuring internal voltage settings...");
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0xA9, &[0x1B]).await;
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0xA8, &[0x6B]).await;
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0xA8, &[0x6D]).await;
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0xA7, &[0x40]).await;
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0xAD, &[0x47]).await;
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0xAF, &[0x73]).await;
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0xAF, &[0x73]).await;
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0xAC, &[0x44]).await;
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0xA3, &[0x6C]).await;
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0xCB, &[0x00]).await;
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0xCD, &[0x22]).await;
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0xC2, &[0x10]).await;
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0xC5, &[0x00]).await;
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0xC6, &[0x0E]).await;
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0xC7, &[0x1F]).await;
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0xC8, &[0x0E]).await;
    info!("Internal voltage settings configured");

    // Single gate mode selection
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0xBF, &[0x00]).await;
    info!("Single gate mode selected");

    // SOU related adjustment
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0xF9, &[0x20]).await;
    info!("SOU adjustment configured");

    // VREG voltage adjustment
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0x9B, &[0x3B]).await;
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0x93, &[0x33, 0x7F, 0x00]).await;
    info!("VREG voltage adjusted");

    // VGH/VGL CLK adjustment
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0x70, &[
        0x0E, 0x0F, 0x03, 0x0E, 0x0F, 0x03
    ]).await;
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0x71, &[
        0x0E, 0x16, 0x03
    ]).await;
    info!("VGH/VGL CLK adjusted");

    // Internal voltage adjustment
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0x91, &[0x0E, 0x09]).await;
    info!("Internal voltage adjusted");

    // VREG voltage adjustment
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0xC3, &[0x2C]).await;
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0xC4, &[0x1A]).await;
    info!("VREG voltage final adjustment");

    // Gamma correction settings
    info!("Configuring gamma correction...");
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0xF0, &[
        0x51, 0x13, 0x0C, 0x06, 0x00, 0x2F
    ]).await;
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0xF2, &[
        0x51, 0x13, 0x0C, 0x06, 0x00, 0x33
    ]).await;
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0xF1, &[
        0x3C, 0x94, 0x4F, 0x33, 0x34, 0xCF
    ]).await;
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0xF3, &[
        0x4D, 0x94, 0x4F, 0x33, 0x34, 0xCF
    ]).await;
    info!("Gamma correction configured");

    // Memory access control
    write_command_with_data(&mut spi, &mut cs_pin, &mut dc_pin, 0x36, &[0x40]).await;
    info!("Memory access control configured");

    // Sleep out command - wake up the display
    write_command(&mut spi, &mut cs_pin, &mut dc_pin, 0x11).await;
    Timer::after_millis(200).await; // Wait for sleep out to complete
    info!("Sleep out command sent");

    // Display on command
    write_command(&mut spi, &mut cs_pin, &mut dc_pin, 0x29).await;
    Timer::after_millis(100).await; // Wait for display to turn on
    info!("Display on command sent");

    // Memory write command (prepare for data)
    write_command(&mut spi, &mut cs_pin, &mut dc_pin, 0x2C).await;
    info!("Memory write command sent");

    info!("GC9D01 complete initialization sequence finished");

    // Display test functionality - RGB color cycling
    info!("Starting display test - RGB color cycling");

    // RGB565 color definitions
    const RED: u16 = 0xF800;     // 11111 000000 00000
    const GREEN: u16 = 0x07E0;   // 00000 111111 00000
    const BLUE: u16 = 0x001F;    // 00000 000000 11111
    const BLACK: u16 = 0x0000;   // 00000 000000 00000

    // GC9D01 is 240x240 pixels
    const DISPLAY_WIDTH: u16 = 240;
    const DISPLAY_HEIGHT: u16 = 240;

    let mut cycle_count = 0u32;

    loop {
        cycle_count += 1;
        info!("=== Starting display test cycle #{} ===", cycle_count);

        // Fill screen with red
        info!("Phase 1/4: Filling screen with red (RGB565: 0x{:04X})...", RED);
        fill_area_with_color(
            &mut spi, &mut cs_pin, &mut dc_pin,
            0, 0, DISPLAY_WIDTH - 1, DISPLAY_HEIGHT - 1,
            RED
        ).await;
        info!("Red fill completed, waiting 3 seconds...");
        Timer::after_secs(3).await;

        // Fill screen with green
        info!("Phase 2/4: Filling screen with green (RGB565: 0x{:04X})...", GREEN);
        fill_area_with_color(
            &mut spi, &mut cs_pin, &mut dc_pin,
            0, 0, DISPLAY_WIDTH - 1, DISPLAY_HEIGHT - 1,
            GREEN
        ).await;
        info!("Green fill completed, waiting 3 seconds...");
        Timer::after_secs(3).await;

        // Fill screen with blue
        info!("Phase 3/4: Filling screen with blue (RGB565: 0x{:04X})...", BLUE);
        fill_area_with_color(
            &mut spi, &mut cs_pin, &mut dc_pin,
            0, 0, DISPLAY_WIDTH - 1, DISPLAY_HEIGHT - 1,
            BLUE
        ).await;
        info!("Blue fill completed, waiting 3 seconds...");
        Timer::after_secs(3).await;

        // Fill screen with black (clear)
        info!("Phase 4/4: Clearing screen with black (RGB565: 0x{:04X})...", BLACK);
        fill_area_with_color(
            &mut spi, &mut cs_pin, &mut dc_pin,
            0, 0, DISPLAY_WIDTH - 1, DISPLAY_HEIGHT - 1,
            BLACK
        ).await;
        info!("Screen cleared, waiting 1 second before next cycle...");
        Timer::after_secs(1).await;

        info!("=== Display test cycle #{} completed successfully ===", cycle_count);
    }
}
