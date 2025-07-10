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
    prelude::*,
};
use gc9d01::{Config as DisplayDriverConfig, GC9D01, Orientation, Timer as Gc9d01Timer};
use static_cell::StaticCell;

use {defmt_rtt as _, panic_probe as _};

use defmt::*;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Starting GC9D01 Example");

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
        // config.enable_ucpd1_dead_battery = true;
    }
    let p = embassy_stm32::init(config);

    struct EmbassyDisplayTimer;
    impl Gc9d01Timer for EmbassyDisplayTimer {
        async fn after_millis(milliseconds: u64) {
            embassy_time::Timer::after_millis(milliseconds).await;
        }
    }

    let spi_peripheral_instance = p.SPI1;
    impl embedded_hal::digital::ErrorType for EmbassyDisplayTimer {
        type Error = core::convert::Infallible;
    }
    let sck_pin = p.PB3;
    let mosi_pin = p.PA7;

    // According to compiler error E0107 (note), Output<'d> has 0 type generic arguments.
    // This contradicts embassy-stm32 source, but we follow the compiler error.
    let cs_pin_output = Output::new(p.PA4, Level::High, Speed::VeryHigh);

    let dc_pin = Output::new(p.PB0, Level::Low, Speed::VeryHigh);
    let rst_pin = Output::new(p.PC4, Level::Low, Speed::VeryHigh);

    let mut spi_config = SpiConfig::default();
    spi_config.frequency = Hertz(16_000_000);

    let spi_bus = Stm32Spi::new_txonly(
        spi_peripheral_instance,
        sck_pin,
        mosi_pin,
        p.DMA1_CH1,
        spi_config,
    );

    // According to compiler error E0107 (note), Spi<'d, M: PeriMode> has 1 type generic argument M.
    // For async SPI1, M should be (peripherals::SPI1, mode::Async).

    static SPI_BUS_CELL: StaticCell<
        Mutex<CriticalSectionRawMutex, Stm32Spi<'static, mode::Async>>,
    > = StaticCell::new();
    let spi_bus_mutex_ref = SPI_BUS_CELL.init(Mutex::new(spi_bus));

    // EmbassySpiDevice<'a, Mtx: RawMutex, BUS: SpiBus, CS: OutputPin>
    // CS type is now CsPinConcreteType = Output<'static>
    let spi_device = EmbassySpiDevice::<
        'static,
        CriticalSectionRawMutex,
        Stm32Spi<'static, mode::Async>,
        Output<'static>,
    >::new(spi_bus_mutex_ref, cs_pin_output);

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

    loop {
        // Clear screen first
        display.fill_color(Rgb565::CSS_BLACK).await.unwrap();
        embassy_time::Timer::after_millis(500).await;

        info!("Drawing two-row test pattern: colors on top, grayscale on bottom.");

        // Define colors for the first row
        let colors = [
            Rgb565::CSS_WHITE,
            Rgb565::CSS_YELLOW,
            Rgb565::CSS_CYAN,
            Rgb565::CSS_GREEN,
            Rgb565::CSS_MAGENTA,
            Rgb565::CSS_RED,
            Rgb565::CSS_BLUE,
            Rgb565::CSS_BLACK,
        ];

        // Define grayscale colors for the second row
        let grayscales = [
            Rgb565::new(31, 63, 31),  // White (max brightness)
            Rgb565::new(27, 54, 27),  // Light gray
            Rgb565::new(23, 46, 23),  // Medium-light gray
            Rgb565::new(19, 38, 19),  // Medium gray
            Rgb565::new(15, 30, 15),  // Medium-dark gray
            Rgb565::new(11, 22, 11),  // Dark gray
            Rgb565::new(7, 14, 7),    // Very dark gray
            Rgb565::new(0, 0, 0),     // Black
        ];

        // Each stripe is 20 pixels wide, each row is 20 pixels high
        const STRIPE_WIDTH: u16 = 20;
        const ROW_HEIGHT: u16 = 20;

        // Create a buffer for one stripe's pixel data
        let mut stripe_pixels = [Rgb565::CSS_BLACK; (STRIPE_WIDTH * ROW_HEIGHT) as usize];

        // Draw first row - color stripes
        info!("Drawing color row (top)");
        for (i, color) in colors.iter().enumerate() {
            let x = i as u16 * STRIPE_WIDTH;
            let y = 0; // First row

            // Fill the stripe buffer with the current color
            for pixel in stripe_pixels.iter_mut() {
                *pixel = *color;
            }

            // Write the pixel data for the current stripe
            display
                .write_area(x, y, STRIPE_WIDTH, ROW_HEIGHT, &stripe_pixels)
                .await
                .unwrap();
        }

        // Draw second row - grayscale stripes
        info!("Drawing grayscale row (bottom)");
        for (i, gray_color) in grayscales.iter().enumerate() {
            let x = i as u16 * STRIPE_WIDTH;
            let y = ROW_HEIGHT; // Second row

            // Fill the stripe buffer with the current grayscale color
            for pixel in stripe_pixels.iter_mut() {
                *pixel = *gray_color;
            }

            // Write the pixel data for the current stripe
            display
                .write_area(x, y, STRIPE_WIDTH, ROW_HEIGHT, &stripe_pixels)
                .await
                .unwrap();
        }

        info!("Two-row pattern completed. Colors: top row, Grayscale: bottom row");
        embassy_time::Timer::after_secs(3).await;
    }
}
