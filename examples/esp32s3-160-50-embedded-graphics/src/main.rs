#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_time::Timer as EmbassyTimer;
use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, Circle, Triangle, Line},
    text::{Baseline, Text},
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
};
use esp_backtrace as _;
use esp_println as _;

use esp_hal::gpio::{Level, Output};
use esp_hal::spi::master::{Config as SpiConfig, Spi};
use esp_hal::spi::Mode as SpiMode;
use esp_hal::timer::timg::TimerGroup;

use gc9d01::{Config as DisplayConfig, GC9D01, Orientation, Timer as Gc9d01Timer};
use static_cell::StaticCell;
use embedded_hal::spi::ErrorType as Eh1ErrorType;
use embedded_hal_async::spi::{Operation as SpiOp, SpiBus as Eh1SpiBus, SpiDevice as Eh1SpiDevice};

// Implement the driver's async Timer using Embassy's timer
struct DisplayTimer;
impl Gc9d01Timer for DisplayTimer {
    async fn after_millis(ms: u64) {
        EmbassyTimer::after_millis(ms).await;
    }
}

// Frame buffer sizing for 160x50 logical area
const LOGICAL_W: usize = 160;
const LOGICAL_H: usize = 50;
const SCREEN_PIXELS: usize = LOGICAL_W * LOGICAL_H;

// Embed ESP-IDF app descriptor required by espflash/bootloader
esp_bootloader_esp_idf::esp_app_desc!();

#[esp_hal_embassy::main]
async fn main(_spawner: Spawner) {
    let p = esp_hal::init(esp_hal::Config::default());
    info!("Starting GC9D01 Embedded-Graphics Demo (ESP32-S3, 160x50 profile)");

    // Embassy time driver init
    let timg0 = TimerGroup::new(p.TIMG0);
    esp_hal_embassy::init(timg0.timer0);

    // SPI setup (SPI2, Mode0, ~10MHz)
    let sclk = p.GPIO12; // SCK
    let mosi = p.GPIO11; // MOSI

    let spi = Spi::new(
        p.SPI2,
        SpiConfig::default()
            .with_frequency(esp_hal::time::Rate::from_hz(10_000_000))
            .with_mode(SpiMode::_0),
    )
    .unwrap()
    .with_sck(sclk)
    .with_mosi(mosi)
    .into_async();

    // Minimal Eh1 SpiDevice wrapper over esp-hal async SPI
    struct SimpleSpiDev<'a, BUS> { bus: BUS, cs: Output<'a> }
    impl<'a, BUS> Eh1ErrorType for SimpleSpiDev<'a, BUS>
    where BUS: Eh1SpiBus<Error = esp_hal::spi::Error>
    { type Error = esp_hal::spi::Error; }
    impl<'a, BUS> Eh1SpiDevice for SimpleSpiDev<'a, BUS>
    where BUS: Eh1SpiBus<Error = esp_hal::spi::Error>
    {
        async fn transaction(&mut self, ops: &mut [SpiOp<'_, u8>]) -> Result<(), Self::Error> {
            self.cs.set_low();
            for op in ops.iter_mut() {
                match op {
                    SpiOp::Write(w) => { self.bus.write(w).await?; }
                    SpiOp::Read(_r) => { core::unimplemented!() }
                    SpiOp::Transfer(_r, _w) => { core::unimplemented!() }
                    SpiOp::TransferInPlace(_b) => { core::unimplemented!() }
                    SpiOp::DelayNs(_ns) => { /* ignore */ }
                }
            }
            let _ = self.cs.set_high();
            Ok(())
        }
    }
    let cs = Output::new(p.GPIO13, Level::High, esp_hal::gpio::OutputConfig::default());
    let spi_dev = SimpleSpiDev { bus: spi, cs };

    // Control pins
    let dc = Output::new(p.GPIO10, Level::Low, esp_hal::gpio::OutputConfig::default());
    let rst = Output::new(p.GPIO14, Level::High, esp_hal::gpio::OutputConfig::default());
    let mut blk = Output::new(p.GPIO15, Level::Low, esp_hal::gpio::OutputConfig::default());
    blk.set_high(); // backlight on

    // Display configuration
    let cfg = DisplayConfig {
        width: LOGICAL_W as u16,
        height: LOGICAL_H as u16,
        orientation: Orientation::Landscape,
        rgb: false,
        inverted: false,
        dx: 15, // panel column offset
        dy: 0,
    };

    // Frame buffer
    static FB: StaticCell<[Rgb565; SCREEN_PIXELS]> = StaticCell::new();
    let fb: &mut [Rgb565] = FB.init([Rgb565::BLACK; SCREEN_PIXELS]);

    let mut disp: GC9D01<_, _, _, DisplayTimer> = GC9D01::new(cfg, spi_dev, dc, rst, fb);

    info!("Initializing panel (panel_160x50 profile)...");
    disp.init().await.ok();

    // Clear
    disp.clear(Rgb565::BLACK).ok();
    disp.flush().await.ok();

    // ========== Demo sequence aligned with stm32 demo, with logs ==========
    // Demo 0: Clear to black
    disp.clear(Rgb565::BLACK).ok();
    disp.flush().await.ok();
    EmbassyTimer::after_millis(300).await;

    // Test 0: Basic fill test (RED, GREEN, BLUE)
    for color in [Rgb565::RED, Rgb565::GREEN, Rgb565::BLUE] {
        // Log color components (5:6:5)
        let raw: u16 = embedded_graphics::pixelcolor::raw::RawU16::from(color).into_inner();
        let _ = raw; // keep variable to avoid warnings when defmt disabled
        disp.clear(color).ok();
        disp.flush().await.ok();
        EmbassyTimer::after_millis(600).await;
    }

    // Pattern 1: Complex multi-color checkerboard (8x? blocks)
    // Pattern 1: Multi-color checkerboard fill (20x20 blocks, partial allowed)
    disp.clear(Rgb565::BLACK).ok();
    let block_w = 20u16;              // 160/8
    let block_h = 20u16;              // vertical step
    let colors = [Rgb565::RED, Rgb565::GREEN, Rgb565::BLUE, Rgb565::YELLOW,
                  Rgb565::MAGENTA, Rgb565::CYAN, Rgb565::WHITE, Rgb565::BLACK];
    let mut row = 0u16;
    while row < LOGICAL_H as u16 {
        let draw_h = core::cmp::min(block_h, (LOGICAL_H as u16) - row);
        for col in 0u16..(LOGICAL_W as u16 / block_w) { // 8 columns exact
            let idx = ((row / block_h) as usize * 8 + col as usize) % colors.len();
            let color = colors[idx];
            let x = col * block_w;
            let y = row;
            Rectangle::new(Point::new(x as i32, y as i32), Size::new(block_w as u32, draw_h as u32))
                .into_styled(PrimitiveStyle::with_fill(color))
                .draw(&mut disp)
                .ok();
        }
        row += block_h;
    }
    disp.flush().await.ok();
    EmbassyTimer::after_millis(900).await;

    // Pattern 2: 10x10 checkerboard
    // Pattern 2: 10x10 checkerboard fill (full screen)
    disp.clear(Rgb565::BLACK).ok();
    let bw = 10i32; let bh = 10i32;
    let nx = (LOGICAL_W as i32 + bw - 1) / bw;
    let ny = (LOGICAL_H as i32 + bh - 1) / bh;
    for r in 0..ny { for c in 0..nx {
        let color = colors[((r + c) as usize) % colors.len()];
        let x = c * bw; let y = r * bh;
        let w = core::cmp::min(bw, LOGICAL_W as i32 - x).max(0) as u32;
        let h = core::cmp::min(bh, LOGICAL_H as i32 - y).max(0) as u32;
        if w > 0 && h > 0 {
            Rectangle::new(Point::new(x, y), Size::new(w, h))
                .into_styled(PrimitiveStyle::with_fill(color))
                .draw(&mut disp).ok();
        }
    }}
    disp.flush().await.ok();
    EmbassyTimer::after_millis(900).await;

    // Demo 1: Basic shapes
    // Demo 1: Basic shapes
    disp.clear(Rgb565::BLACK).ok();
    Rectangle::new(Point::new(10, 8), Size::new(30, 15))
        .into_styled(PrimitiveStyle::with_stroke(Rgb565::RED, 1))
        .draw(&mut disp).ok();
    Rectangle::new(Point::new(50, 12), Size::new(25, 12))
        .into_styled(PrimitiveStyle::with_fill(Rgb565::GREEN))
        .draw(&mut disp).ok();
    Circle::new(Point::new(90, 16), 15)
        .into_styled(PrimitiveStyle::with_stroke(Rgb565::BLUE, 2))
        .draw(&mut disp).ok();
    Circle::new(Point::new(120, 18), 8)
        .into_styled(PrimitiveStyle::with_fill(Rgb565::YELLOW))
        .draw(&mut disp).ok();
    disp.flush().await.ok();
    EmbassyTimer::after_millis(900).await;

    // Demo 2: Lines
    // Demo 2: Lines
    disp.clear(Rgb565::BLACK).ok();
    for i in 0..8 {
        let color = match i % 3 { 0 => Rgb565::RED, 1 => Rgb565::GREEN, _ => Rgb565::BLUE };
        Line::new(Point::new(i * 20, 0), Point::new(i * 20 + 20, (LOGICAL_H as i32) - 1))
            .into_styled(PrimitiveStyle::with_stroke(color, 1))
            .draw(&mut disp).ok();
    }
    disp.flush().await.ok();
    EmbassyTimer::after_millis(900).await;

    // Demo 3: Text
    // Demo 3: Text
    disp.clear(Rgb565::BLACK).ok();
    let ts = MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE);
    Text::with_baseline("GC9D01", Point::new(8, 10), ts, Baseline::Top).draw(&mut disp).ok();
    Text::with_baseline("Graphics", Point::new(8, 26), ts, Baseline::Top).draw(&mut disp).ok();
    disp.flush().await.ok();
    EmbassyTimer::after_millis(900).await;

    // Demo 4: Triangles
    // Demo 4: Triangles
    disp.clear(Rgb565::BLACK).ok();
    Triangle::new(Point::new(24, 8), Point::new(12, 34), Point::new(36, 34))
        .into_styled(PrimitiveStyle::with_fill(Rgb565::MAGENTA))
        .draw(&mut disp).ok();
    Triangle::new(Point::new(70, 12), Point::new(54, 42), Point::new(86, 42))
        .into_styled(PrimitiveStyle::with_stroke(Rgb565::CYAN, 2))
        .draw(&mut disp).ok();
    disp.flush().await.ok();
    EmbassyTimer::after_millis(900).await;

    // Demo 5: Square grid (10x10) — ensure square cells and full coverage
    // Demo 5: Square grid (10x10)
    disp.clear(Rgb565::BLACK).ok();
    let s = 10i32; // square size
    let cols = (LOGICAL_W as i32 + s - 1) / s;
    let rows = (LOGICAL_H as i32 + s - 1) / s;
    let _ = (cols, rows, s); // suppress unused when logging is off
    for cx in 0..cols {
        for cy in 0..rows {
            let x = cx * s; let y = cy * s;
            let w = core::cmp::min(s, LOGICAL_W as i32 - x).max(0) as u32;
            let h = core::cmp::min(s, LOGICAL_H as i32 - y).max(0) as u32;
            if w == 0 || h == 0 { continue; }
            let color = if ((cx + cy) % 2) == 0 { Rgb565::WHITE } else { Rgb565::BLACK };
            Rectangle::new(Point::new(x, y), Size::new(w, h))
                .into_styled(PrimitiveStyle::with_fill(color))
                .draw(&mut disp).ok();
        }
    }
    disp.flush().await.ok();

    // Idle
    loop { EmbassyTimer::after_millis(1000).await; }
}
