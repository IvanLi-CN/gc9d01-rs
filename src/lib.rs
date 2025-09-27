#![no_std]

// NOTE: Due to limitations in mocking interactions with ownership and the `Drop` trait
// when using `embedded-hal-mock` (specifically with its `DoneCallDetector`),
// stable and consistently passing unit/integration tests for the SPI interactions
// are not provided with this library version. Users are advised to perform
// thorough integration testing in their target hardware environment.
//
// Previous attempts to create robust mock-based tests for synchronous SPI operations
// encountered persistent issues with `DoneCallDetector` panics, stemming from
// the inability to call the mock's `.done()` method before it's dropped when
// owned by the driver struct, a common pattern in embedded drivers.

// Assuming "async" feature is always on for this simplified test
use core::marker::PhantomData;

use embedded_graphics_core::Pixel as EgPixel;
use embedded_graphics_core::pixelcolor::Rgb565;
use embedded_graphics_core::pixelcolor::raw::RawData;
use embedded_graphics_core::prelude::{DrawTarget, OriginDimensions, Size};
use embedded_hal::digital::OutputPin;
use embedded_hal::spi::Error as SpiError; // Directly use async SpiBus

#[cfg(not(feature = "async"))]
use embedded_hal::spi::SpiDevice;
#[cfg(feature = "async")]
use embedded_hal_async::spi::SpiDevice;

// Timer trait now only needs async version for this test

#[maybe_async_cfg::maybe(
    sync(cfg(not(feature = "async")), self = "Timer",),
    async(feature = "async", keep_self)
)]
pub trait Timer {
    /// Expire after the specified number of milliseconds.
    fn after_millis(milliseconds: u64) -> impl Future<Output = ()>;
}

// Back-compat framebuffer constants (deprecated).
// These were previously exported and some downstream code may still depend on them
// for sizing buffers. Keep them exported to avoid a breaking change while
// providing better alternatives on `Config` below.
#[deprecated(
    since = "0.0.2",
    note = "Use Config::frame_bytes() or Config::frame_pixels() instead; this constant will be removed in a future release."
)]
pub const FRAME_BUF_SIZE: usize = 160 * 40 * 2; // bytes (default profile 160x40)
#[deprecated(
    since = "0.0.2",
    note = "Use Config::frame_pixels() instead; this constant will be removed in a future release."
)]
pub const MAX_FRAME_PIXELS: usize = 160 * 40; // pixels (default profile 160x40)

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum Instruction {
    Nop = 0x00,
    SwReset = 0x01,
    ReadDisplayId = 0x04,
    ReadDisplayStatus = 0x09,
    ReadDisplayPowerMode = 0x0A,
    ReadDisplayMadctl = 0x0B,
    ReadDisplayPixelFormat = 0x0C,
    ReadDisplayImageMode = 0x0D,
    ReadDisplaySignalMode = 0x0E,
    ReadDisplaySelfDiagnosticResult = 0x0F,
    SleepIn = 0x10,
    SleepOut = 0x11,
    PartialModeOn = 0x12,
    NormalDisplayOn = 0x13,
    DisplayInversionOff = 0x20,
    DisplayInversionOn = 0x21,
    DisplayOff = 0x28,
    DisplayOn = 0x29,
    ColumnAddressSet = 0x2A,
    RowAddressSet = 0x2B,
    MemoryWrite = 0x2C,
    MemoryRead = 0x2E,
    PartialArea = 0x30,
    VerticalScrollingDefinition = 0x33,
    TearingEffectLineOff = 0x34,
    TearingEffectLineOn = 0x35,
    MemoryAccessControl = 0x36,
    VerticalScrollingStartAddress = 0x37,
    IdleModeOff = 0x38,
    IdleModeOn = 0x39,
    PixelFormatSet = 0x3A,
    WriteMemoryContinue = 0x3C,
    ReadMemoryContinue = 0x3E,
    SetTearScanline = 0x44,
    GetScanline = 0x45,
    WriteDisplayBrightness = 0x51,
    ReadDisplayBrightness = 0x52,
    WriteCtrlDisplay = 0x53,
    ReadCtrlDisplay = 0x54,
    WriteCabc = 0x55,
    ReadCabc = 0x56,
    WriteCabcMinBrightness = 0x5E,
    ReadCabcMinBrightness = 0x5F,
    RgbInterfaceSignalControl = 0xB0,
    Spi2DataControl = 0xB1,
    TearingEffectControl = 0xB4,
    BlankingPorchControl = 0xB5,
    DisplayFunctionControl = 0xB6,
    DualSingleGateSelect = 0xBF,
    PowerControl1 = 0xC1,
    PowerControl2 = 0xC3,
    PowerControl3 = 0xC4,
    PowerControl4 = 0xC9,
    ReadId1 = 0xDA,
    ReadId2 = 0xDB,
    ReadId3 = 0xDC,
    Inversion = 0xEC,
    InterRegisterEnable2 = 0xEF,
    SetGamma1 = 0xF0,
    SetGamma2 = 0xF1,
    SetGamma3 = 0xF2,
    SetGamma4 = 0xF3,
    InterfaceControl = 0xF6,
    InterRegisterEnable1 = 0xFE,
    Cmd80 = 0x80,
    Cmd81 = 0x81,
    Cmd82 = 0x82,
    Cmd83 = 0x83,
    Cmd84 = 0x84,
    Cmd85 = 0x85,
    Cmd86 = 0x86,
    Cmd87 = 0x87,
    Cmd88 = 0x88,
    Cmd89 = 0x89,
    Cmd8A = 0x8A,
    Cmd8B = 0x8B,
    Cmd8C = 0x8C,
    Cmd8D = 0x8D,
    Cmd8E = 0x8E,
    Cmd8F = 0x8F,
    Cmd7E = 0x7E,
    Cmd74 = 0x74,
    Cmd98 = 0x98,
    Cmd99 = 0x99,
    Cmd60 = 0x60,
    Cmd63 = 0x63,
    Cmd64 = 0x64,
    Cmd66 = 0x66,
    Cmd6A = 0x6A,
    Cmd68 = 0x68,
    Cmd6C = 0x6C,
    Cmd6E = 0x6E,
    CmdA9 = 0xA9,
    CmdA8 = 0xA8,
    CmdA7 = 0xA7,
    CmdAD = 0xAD,
    CmdAF = 0xAF,
    CmdAC = 0xAC,
    CmdA3 = 0xA3,
    CmdCB = 0xCB,
    CmdCD = 0xCD,
    CmdC2 = 0xC2,
    CmdC5 = 0xC5,
    CmdC6 = 0xC6,
    CmdC7 = 0xC7,
    CmdC8 = 0xC8,
    CmdF9 = 0xF9,
    Cmd9B = 0x9B,
    Cmd93 = 0x93,
    Cmd70 = 0x70,
    Cmd71 = 0x71,
    Cmd91 = 0x91,
}

#[derive(Clone, Copy)]
pub enum Orientation {
    Portrait = 0x00,
    Landscape = 0x60,
    PortraitSwapped = 0x80,
    LandscapeSwapped = 0xA0,
}

#[derive(Clone, Copy)]
pub struct Config {
    pub rgb: bool,
    pub inverted: bool,
    pub orientation: Orientation,
    pub height: u16,
    pub width: u16,
    pub dx: u16,
    pub dy: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            rgb: false,
            inverted: false,
            // Default to 160x40 profile (logical rendering area)
            orientation: Orientation::Portrait,
            height: 40,
            width: 160,
            dx: 0,
            dy: 0,
        }
    }
}

impl Config {
    /// Total pixels of the frame (width * height).
    pub const fn frame_pixels(&self) -> usize {
        (self.width as usize) * (self.height as usize)
    }

    /// Total bytes of the frame buffer for RGB565 (2 bytes per pixel).
    pub const fn frame_bytes(&self) -> usize {
        self.frame_pixels() * 2
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error<BusE, PinE>
where
    BusE: core::fmt::Debug,
    PinE: core::fmt::Debug,
{
    Bus(BusE),
    Pin(PinE),
}

pub struct GC9D01<'b, BUS, DC, RST, TIMER>
where
    BUS: SpiDevice, // Will be async SpiBus
    DC: OutputPin,
    RST: OutputPin,
    TIMER: crate::Timer, // Timer trait is now always async
{
    bus: BUS,
    dc: DC,
    rst: RST,
    config: Config,
    frame_buffer: &'b mut [Rgb565], // Required frame buffer for full-screen rendering
    _timer: PhantomData<TIMER>,
}

#[maybe_async_cfg::maybe(
    sync(cfg(not(feature = "async")), self = "GC9D01",),
    async(feature = "async", keep_self)
)]
impl<'b, BUS, DC, RST, TIMER, BusE, PinE> GC9D01<'b, BUS, DC, RST, TIMER>
where
    BUS: SpiDevice<Error = BusE>,
    DC: OutputPin<Error = PinE>,
    RST: OutputPin<Error = PinE>,
    TIMER: crate::Timer,
    BusE: core::fmt::Debug + SpiError,
    PinE: core::fmt::Debug,
{
    /// Convert Rgb565 color to correct byte order for GC9D01 display
    /// GC9D01 expects RGB565 data in big-endian format (high byte first)
    fn convert_color_byte_order(color: Rgb565) -> Rgb565 {
        // Convert Rgb565 to RawU16 to get the raw u16 value
        let raw_pixel: embedded_graphics_core::pixelcolor::raw::RawU16 = color.into();
        let pixel_value: u16 = raw_pixel.into_inner();

        // Swap bytes: convert from little-endian to big-endian
        let swapped_value = pixel_value.swap_bytes();

        // Convert back to Rgb565
        embedded_graphics_core::pixelcolor::raw::RawU16::new(swapped_value).into()
    }
    /// Create a new GC9D01 instance with frame buffer support for full-screen rendering
    pub fn new(config: Config, bus: BUS, dc: DC, rst: RST, frame_buffer: &'b mut [Rgb565]) -> Self {
        Self {
            bus,
            dc,
            rst,
            config,
            frame_buffer,
            _timer: PhantomData,
        }
    }

    pub async fn init(&mut self) -> Result<(), Error<BusE, PinE>> {
        self.reset().await?; // 标准硬件复位

        self.write_command(Instruction::InterRegisterEnable1, &[])
            .await?; // 0xFE
        self.write_command(Instruction::InterRegisterEnable2, &[])
            .await?; // 0xEF

        // 内部寄存器使能 80~8Fh
        self.write_command(Instruction::Cmd80, &[0xFF]).await?;
        self.write_command(Instruction::Cmd81, &[0xFF]).await?;
        self.write_command(Instruction::Cmd82, &[0xFF]).await?;
        self.write_command(Instruction::Cmd83, &[0xFF]).await?;
        self.write_command(Instruction::Cmd84, &[0xFF]).await?;
        self.write_command(Instruction::Cmd85, &[0xFF]).await?;
        self.write_command(Instruction::Cmd86, &[0xFF]).await?;
        self.write_command(Instruction::Cmd87, &[0xFF]).await?;
        self.write_command(Instruction::Cmd88, &[0xFF]).await?;
        self.write_command(Instruction::Cmd89, &[0xFF]).await?;
        self.write_command(Instruction::Cmd8A, &[0xFF]).await?;
        self.write_command(Instruction::Cmd8B, &[0xFF]).await?;
        self.write_command(Instruction::Cmd8C, &[0xFF]).await?;
        self.write_command(Instruction::Cmd8D, &[0xFF]).await?;
        self.write_command(Instruction::Cmd8E, &[0xFF]).await?;
        self.write_command(Instruction::Cmd8F, &[0xFF]).await?;

        self.write_command(Instruction::PixelFormatSet, &[0x05])
            .await?; // 0x3A

        // VGL size (panel-specific)
        #[cfg(not(feature = "panel_160x50"))]
        self.write_command(Instruction::Cmd7E, &[0x7A]).await?; // reference profile
        #[cfg(feature = "panel_160x50")]
        self.write_command(Instruction::Cmd7E, &[0x30]).await?; // narrow module profile

        // 修改帧频
        // Frame frequency (panel-specific)
        #[cfg(not(feature = "panel_160x50"))]
        self.write_command(
            Instruction::Cmd74,
            &[0x02, 0x0E, 0x00, 0x00, 0x28, 0x00, 0x00],
        )
        .await?;
        #[cfg(feature = "panel_160x50")]
        self.write_command(
            Instruction::Cmd74,
            &[0x05, 0x4D, 0x00, 0x00, 0x01, 0x00, 0x00],
        )
        .await?;

        // 内部电压调整
        self.write_command(Instruction::Cmd98, &[0x3E]).await?;
        self.write_command(Instruction::Cmd99, &[0x3E]).await?;

        // Porch (panel-specific)
        #[cfg(not(feature = "panel_160x50"))]
        self.write_command(Instruction::BlankingPorchControl, &[0x0E, 0x0E])
            .await?; // 0xB5
        #[cfg(feature = "panel_160x50")]
        self.write_command(Instruction::BlankingPorchControl, &[0x0D, 0x0D])
            .await?; // 0xB5

        // gip timing start
        // GIP timing (panel-specific)
        #[cfg(not(feature = "panel_160x50"))]
        {
            self.write_command(Instruction::Cmd60, &[0x38, 0x09, 0x6D, 0x67])
                .await?;
            self.write_command(Instruction::Cmd63, &[0x38, 0xAD, 0x6D, 0x67, 0x05])
                .await?;
            self.write_command(Instruction::Cmd64, &[0x38, 0x0B, 0x70, 0xAB, 0x6D, 0x67])
                .await?;
            self.write_command(Instruction::Cmd66, &[0x38, 0x0F, 0x70, 0xAF, 0x6D, 0x67])
                .await?;
            self.write_command(Instruction::Cmd6A, &[0x00, 0x00])
                .await?;
            self.write_command(
                Instruction::Cmd68,
                &[0x3B, 0x08, 0x04, 0x00, 0x04, 0x64, 0x67],
            )
            .await?;
            self.write_command(
                Instruction::Cmd6C,
                &[0x22, 0x02, 0x22, 0x02, 0x22, 0x22, 0x50],
            )
            .await?;
            self.write_command(
                Instruction::Cmd6E,
                &[
                    0x00, 0x00, 0x00, 0x00, 0x07, 0x01, 0x13, 0x11, 0x0B, 0x09, 0x16, 0x15, 0x1D,
                    0x1E, 0x00, 0x00, 0x00, 0x00, 0x1E, 0x1D, 0x15, 0x16, 0x0A, 0x0C, 0x12, 0x14,
                    0x02, 0x08, 0x00, 0x00, 0x00, 0x00,
                ],
            )
            .await?;
        }
        #[cfg(feature = "panel_160x50")]
        {
            self.write_command(Instruction::Cmd60, &[0x38, 0x09, 0x1E, 0x7A])
                .await?;
            self.write_command(Instruction::Cmd63, &[0x38, 0xAE, 0x1E, 0x7A])
                .await?;
            self.write_command(Instruction::Cmd64, &[0x38, 0x0B, 0x70, 0xAB, 0x1E, 0x7A])
                .await?;
            self.write_command(Instruction::Cmd66, &[0x38, 0x0F, 0x70, 0xAF, 0x1E, 0x7A])
                .await?;
            self.write_command(
                Instruction::Cmd68,
                &[0x00, 0x08, 0x07, 0x00, 0x07, 0x55, 0x6A],
            )
            .await?;
            self.write_command(Instruction::Cmd6A, &[0x00, 0x00])
                .await?;
            self.write_command(
                Instruction::Cmd6C,
                &[0x22, 0x02, 0x22, 0x02, 0x22, 0x22, 0x50],
            )
            .await?;
            self.write_command(
                Instruction::Cmd6E,
                &[
                    0x00, 0x00, 0x00, 0x02, 0x14, 0x12, 0x0C, 0x0A, 0x1E, 0x1D, 0x08, 0x00, 0x16,
                    0x15, 0x00, 0x00, 0x00, 0x00, 0x15, 0x16, 0x00, 0x07, 0x1D, 0x1E, 0x09, 0x0B,
                    0x11, 0x13, 0x01, 0x00, 0x00, 0x00,
                ],
            )
            .await?;
        }
        // gip timing end

        // Internal voltage settings block (present on reference profile only)
        #[cfg(not(feature = "panel_160x50"))]
        {
            self.write_command(Instruction::CmdA9, &[0x1B]).await?;
            self.write_command(Instruction::CmdA8, &[0x6B]).await?;
            self.write_command(Instruction::CmdA8, &[0x6D]).await?;
            self.write_command(Instruction::CmdA7, &[0x40]).await?;
            self.write_command(Instruction::CmdAD, &[0x47]).await?;
            self.write_command(Instruction::CmdAF, &[0x73]).await?;
            self.write_command(Instruction::CmdAF, &[0x73]).await?;
            self.write_command(Instruction::CmdAC, &[0x44]).await?;
            self.write_command(Instruction::CmdA3, &[0x6C]).await?;
            self.write_command(Instruction::CmdCB, &[0x00]).await?;
            self.write_command(Instruction::CmdCD, &[0x22]).await?;
            self.write_command(Instruction::CmdC2, &[0x10]).await?;
            self.write_command(Instruction::CmdC5, &[0x00]).await?;
            self.write_command(Instruction::CmdC6, &[0x0E]).await?;
            self.write_command(Instruction::CmdC7, &[0x1F]).await?;
            self.write_command(Instruction::CmdC8, &[0x0E]).await?;
        }

        // Dual-Single gate select (common)
        self.write_command(Instruction::DualSingleGateSelect, &[0x00])
            .await?; // 0xBF

        // SOU (panel-specific)
        #[cfg(not(feature = "panel_160x50"))]
        self.write_command(Instruction::CmdF9, &[0x20]).await?;
        #[cfg(feature = "panel_160x50")]
        self.write_command(Instruction::CmdF9, &[0x40]).await?;

        // vreg / clock / internal voltage tuning (panel-specific portions)
        self.write_command(Instruction::Cmd9B, &[0x3B]).await?;
        self.write_command(Instruction::Cmd93, &[0x33, 0x7F, 0x00])
            .await?;
        self.write_command(Instruction::Cmd91, &[0x0E, 0x09])
            .await?;

        #[cfg(not(feature = "panel_160x50"))]
        {
            self.write_command(Instruction::Cmd70, &[0x0E, 0x0F, 0x03, 0x0E, 0x0F, 0x03])
                .await?;
            self.write_command(Instruction::Cmd71, &[0x0E, 0x16, 0x03])
                .await?;
            self.write_command(Instruction::PowerControl2, &[0x2C])
                .await?; // 0xC3
            self.write_command(Instruction::PowerControl3, &[0x1A])
                .await?; // 0xC4
        }
        #[cfg(feature = "panel_160x50")]
        {
            self.write_command(Instruction::Cmd70, &[0x04, 0x02, 0x0D, 0x04, 0x02, 0x0D])
                .await?;
            self.write_command(Instruction::Cmd71, &[0x04, 0x02, 0x0D])
                .await?;
            self.write_command(Instruction::PowerControl2, &[0x26])
                .await?; // 0xC3
            self.write_command(Instruction::PowerControl3, &[0x26])
                .await?; // 0xC4
            self.write_command(Instruction::PowerControl4, &[0x1C])
                .await?; // 0xC9
        }

        // gamma F0~F3h (注意伪代码中F0, F2, F1, F3的顺序)
        // Gamma (panel-specific)
        #[cfg(not(feature = "panel_160x50"))]
        {
            self.write_command(
                Instruction::SetGamma1,
                &[0x51, 0x13, 0x0C, 0x06, 0x00, 0x2F],
            )
            .await?;
            self.write_command(
                Instruction::SetGamma3,
                &[0x51, 0x13, 0x0C, 0x06, 0x00, 0x33],
            )
            .await?;
            self.write_command(
                Instruction::SetGamma2,
                &[0x3C, 0x94, 0x4F, 0x33, 0x34, 0xCF],
            )
            .await?;
            self.write_command(
                Instruction::SetGamma4,
                &[0x4D, 0x94, 0x4F, 0x33, 0x34, 0xCF],
            )
            .await?;
        }
        #[cfg(feature = "panel_160x50")]
        {
            self.write_command(
                Instruction::SetGamma1,
                &[0x02, 0x03, 0x0A, 0x06, 0x00, 0x1A],
            )
            .await?;
            self.write_command(
                Instruction::SetGamma3,
                &[0x02, 0x03, 0x0A, 0x06, 0x00, 0x1A],
            )
            .await?;
            self.write_command(
                Instruction::SetGamma2,
                &[0x38, 0x78, 0x1B, 0x2E, 0x2F, 0xC8],
            )
            .await?;
            self.write_command(
                Instruction::SetGamma4,
                &[0x38, 0x74, 0x12, 0x2E, 0x2F, 0xDF],
            )
            .await?;
        }

        // Inversion (0xEC) differs
        #[cfg(not(feature = "panel_160x50"))]
        self.write_command(Instruction::Inversion, &[0x11]).await?;
        #[cfg(feature = "panel_160x50")]
        self.write_command(Instruction::Inversion, &[0x00]).await?;

        // Memory access control (panel-specific)
        #[cfg(not(feature = "panel_160x50"))]
        self.write_command(Instruction::MemoryAccessControl, &[0x40])
            .await?; // 0x36
        #[cfg(feature = "panel_160x50")]
        self.write_command(Instruction::MemoryAccessControl, &[0x00])
            .await?; // 0x36

        // For the 160x50 narrow module, set initial address window to match the
        // vendor sample exactly: columns 0x000F..0x0040 (50px with X_OFFSET=15),
        // rows 0x0000..0x009F (160px), then proceed to sleep-out/display-on.
        #[cfg(feature = "panel_160x50")]
        {
            let sx: u16 = self.config.dx; // 15
            let ex: u16 = self.config.dx + 50 - 1; // 15+49=64
            let sy: u16 = self.config.dy; // 0
            let ey: u16 = self.config.dy + 160 - 1; // 159
            self.write_command(
                Instruction::ColumnAddressSet,
                &[(sx >> 8) as u8, sx as u8, (ex >> 8) as u8, ex as u8],
            )
            .await?;
            self.write_command(
                Instruction::RowAddressSet,
                &[(sy >> 8) as u8, sy as u8, (ey >> 8) as u8, ey as u8],
            )
            .await?;
        }

        // Display function control only for reference-style profile
        #[cfg(not(feature = "panel_160x50"))]
        {
            self.write_command(
                Instruction::DisplayFunctionControl,
                &[0x0A, 0x80, 0x27, 0x00],
            )
            .await?; // 0xB6
        }

        self.write_command(Instruction::SleepOut, &[]).await?; // 0x11
        #[allow(unused_must_use)]
        {
            TIMER::after_millis(200).await;
        }
        self.write_command(Instruction::DisplayOn, &[]).await?; // 0x29

        // Memory write command - ready for pixel data (as in working example)
        self.write_command(Instruction::MemoryWrite, &[]).await?; // 0x2C
        #[allow(unused_must_use)]
        {
            TIMER::after_millis(100).await;
        }

        Ok(()) // init function ends here
    }

    pub async fn reset(&mut self) -> Result<(), Error<BusE, PinE>> {
        // 硬件复位序列: RST 拉低 -> 延时10ms -> RST 拉高 -> 延时120ms
        self.rst.set_low().map_err(Error::Pin)?;
        #[allow(unused_must_use)]
        {
            TIMER::after_millis(10).await;
        }
        self.rst.set_high().map_err(Error::Pin)?;
        #[allow(unused_must_use)]
        {
            TIMER::after_millis(120).await;
        }
        Ok(())
    }

    async fn write_command(
        &mut self,
        instruction: Instruction,
        params: &[u8],
    ) -> Result<(), Error<BusE, PinE>> {
        self.dc.set_low().map_err(Error::Pin)?;

        let cmd_bytes = [instruction as u8];
        let cmd_res = self.bus.write(&cmd_bytes).await.map_err(Error::Bus);

        if cmd_res.is_ok() && !params.is_empty() {
            self.dc.set_high().map_err(Error::Pin)?;
            let param_res = self.bus.write(params).await.map_err(Error::Bus);
            if param_res.is_err() {
                param_res
            } else {
                Ok(())
            }
        } else if cmd_res.is_err() {
            cmd_res
        } else {
            Ok(())
        }
    }

    fn start_data_internal(&mut self) -> Result<(), PinE> {
        self.dc.set_high()
    }

    /// Transform logical coordinates to physical coordinates based on orientation
    fn transform_coordinates(&self, x: u16, y: u16) -> (u16, u16) {
        let (phys_w, phys_h) = self.physical_dimensions();
        match self.config.orientation {
            Orientation::Portrait => (phys_w - 1 - y, phys_h - 1 - x),
            Orientation::Landscape => (y, phys_h - 1 - x),
            Orientation::PortraitSwapped => (x, y),
            Orientation::LandscapeSwapped => (y, phys_h - 1 - x),
        }
    }

    fn physical_dimensions(&self) -> (u16, u16) {
        #[cfg(feature = "panel_160x50")]
        {
            // Narrow module: physical addressing is 50 (columns) x 160 (rows)
            // Column addressing (2Ah) uses the short side with an X offset.
            (50, 160)
        }
        #[cfg(not(feature = "panel_160x50"))]
        {
            // Reference-style panel: physical matches logical
            (self.config.width, self.config.height)
        }
    }

    pub async fn set_address_window(
        &mut self,
        sx: u16,
        sy: u16,
        ex: u16,
        ey: u16,
    ) -> Result<(), Error<BusE, PinE>> {
        // Apply coordinate transformation based on orientation
        let (phys_sx, phys_sy) = self.transform_coordinates(sx, sy);
        let (phys_ex, phys_ey) = self.transform_coordinates(ex, ey);

        // Apply offset adjustments
        let final_sx = phys_sx + self.config.dx;
        let final_ex = phys_ex + self.config.dx;
        let final_sy = phys_sy + self.config.dy;
        let final_ey = phys_ey + self.config.dy;

        // Ensure coordinates are in correct order
        let (min_x, max_x) = if final_sx <= final_ex {
            (final_sx, final_ex)
        } else {
            (final_ex, final_sx)
        };
        let (min_y, max_y) = if final_sy <= final_ey {
            (final_sy, final_ey)
        } else {
            (final_ey, final_sy)
        };

        #[cfg(feature = "defmt")]
        defmt::debug!(
            "Address window: logical ({},{}) to ({},{}) -> physical ({},{}) to ({},{})",
            sx,
            sy,
            ex,
            ey,
            min_x,
            min_y,
            max_x,
            max_y
        );

        self.write_command(
            Instruction::RowAddressSet,
            &[
                (min_x >> 8) as u8,
                min_x as u8,
                (max_x >> 8) as u8,
                max_x as u8,
            ],
        )
        .await?;
        self.write_command(
            Instruction::ColumnAddressSet,
            &[
                (min_y >> 8) as u8,
                min_y as u8,
                (max_y >> 8) as u8,
                max_y as u8,
            ],
        )
        .await
    }

    // Frame buffer operations

    /// Clear the frame buffer with a solid color
    pub fn clear_frame_buffer(&mut self, color: Rgb565) {
        // Convert color to correct byte order for GC9D01 display
        let converted_color = Self::convert_color_byte_order(color);
        for pixel in self.frame_buffer.iter_mut() {
            *pixel = converted_color;
        }
    }

    /// Set a pixel in the frame buffer
    pub fn set_pixel(&mut self, x: u16, y: u16, color: Rgb565) {
        if x < self.config.width && y < self.config.height {
            let converted_color = Self::convert_color_byte_order(color);
            let (px, py) = self.transform_coordinates(x, y);
            let (phys_w, _phys_h) = self.physical_dimensions();
            let index = (py as usize) * (phys_w as usize) + (px as usize);
            if index < self.frame_buffer.len() {
                self.frame_buffer[index] = converted_color;
            }
        }
    }

    /// Fill a rectangular area in the frame buffer
    pub fn fill_rect(&mut self, x: u16, y: u16, width: u16, height: u16, color: Rgb565) {
        // Convert color to correct byte order for GC9D01 display
        let converted_color = Self::convert_color_byte_order(color);

        for row in y..(y + height) {
            for col in x..(x + width) {
                if col < self.config.width && row < self.config.height {
                    let (px, py) = self.transform_coordinates(col, row);
                    let (phys_w, _phys_h) = self.physical_dimensions();
                    let index = (py as usize) * (phys_w as usize) + (px as usize);
                    if index < self.frame_buffer.len() {
                        self.frame_buffer[index] = converted_color;
                    }
                }
            }
        }
    }

    /// Write pixel data to a rectangular area in the frame buffer
    pub fn write_rect(&mut self, x: u16, y: u16, width: u16, height: u16, data: &[Rgb565]) {
        let mut data_index = 0;
        for row in y..(y + height) {
            for col in x..(x + width) {
                if col < self.config.width && row < self.config.height && data_index < data.len() {
                    let converted_color = Self::convert_color_byte_order(data[data_index]);
                    let (px, py) = self.transform_coordinates(col, row);
                    let (phys_w, _phys_h) = self.physical_dimensions();
                    let index = (py as usize) * (phys_w as usize) + (px as usize);
                    if index < self.frame_buffer.len() {
                        self.frame_buffer[index] = converted_color;
                    }
                    data_index += 1;
                }
            }
        }
    }

    /// Flush the frame buffer to the display
    pub async fn flush(&mut self) -> Result<(), Error<BusE, PinE>> {
        // Set address window for the entire physical screen
        let (phys_w, phys_h) = self.physical_dimensions();
        // Use configured dx/dy offsets so the flush window matches the panel's
        // active region (important for 160x50 modules with X offset).
        let sx = self.config.dx;
        let sy = self.config.dy;
        let ex = sx + phys_w - 1;
        let ey = sy + phys_h - 1;
        self.write_command(
            Instruction::ColumnAddressSet,
            &[(sx >> 8) as u8, sx as u8, (ex >> 8) as u8, ex as u8],
        )
        .await?;
        self.write_command(
            Instruction::RowAddressSet,
            &[(sy >> 8) as u8, sy as u8, (ey >> 8) as u8, ey as u8],
        )
        .await?;
        self.write_command(Instruction::MemoryWrite, &[]).await?;

        // Start data transmission
        self.start_data_internal().map_err(Error::Pin)?;

        // Send frame buffer data in chunks to respect DMA limitations
        // Convert frame_buffer to bytes view without copying
        let frame_bytes = unsafe {
            core::slice::from_raw_parts(
                self.frame_buffer.as_ptr() as *const u8,
                self.frame_buffer.len() * 2,
            )
        };

        // Send in chunks respecting DMA 16-bit counter limit (max 65535 bytes)
        // Use conservative 4096 bytes (2048 pixels) for safety
        const CHUNK_SIZE: usize = 4096;
        let mut offset = 0;

        while offset < frame_bytes.len() {
            let chunk_end = core::cmp::min(offset + CHUNK_SIZE, frame_bytes.len());
            self.bus
                .write(&frame_bytes[offset..chunk_end])
                .await
                .map_err(Error::Bus)?;
            offset = chunk_end;
        }

        Ok(())
    }

    pub fn fill_color(&mut self, color: Rgb565) {
        // Only operate on frame buffer - no hardware operations
        // clear_frame_buffer already handles color conversion
        self.clear_frame_buffer(color);
    }

    pub fn write_area(&mut self, x: u16, y: u16, width: u16, height: u16, data: &[Rgb565]) {
        // Only operate on frame buffer - no hardware operations
        // write_rect already handles color conversion
        self.write_rect(x, y, width, height, data);
    }
}

// Embedded Graphics trait implementations
impl<BUS, DC, RST, TIMER, BusE, PinE> OriginDimensions for GC9D01<'_, BUS, DC, RST, TIMER>
where
    BUS: SpiDevice<Error = BusE>,
    DC: OutputPin<Error = PinE>,
    RST: OutputPin<Error = PinE>,
    TIMER: crate::Timer,
    BusE: core::fmt::Debug + SpiError,
    PinE: core::fmt::Debug,
{
    fn size(&self) -> Size {
        Size::new(self.config.width as u32, self.config.height as u32)
    }
}

impl<BUS, DC, RST, TIMER, BusE, PinE> DrawTarget for GC9D01<'_, BUS, DC, RST, TIMER>
where
    BUS: SpiDevice<Error = BusE>,
    DC: OutputPin<Error = PinE>,
    RST: OutputPin<Error = PinE>,
    TIMER: crate::Timer,
    BusE: core::fmt::Debug + SpiError,
    PinE: core::fmt::Debug,
{
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = EgPixel<Self::Color>>,
    {
        for EgPixel(point, color) in pixels {
            if point.x >= 0 && point.y >= 0 {
                let x = point.x as u16;
                let y = point.y as u16;
                if x < self.config.width && y < self.config.height {
                    self.set_pixel(x, y, color);
                }
            }
        }
        Ok(())
    }

    fn clear(&mut self, color: Self::Color) -> Result<(), Self::Error> {
        self.clear_frame_buffer(color);
        Ok(())
    }
}
