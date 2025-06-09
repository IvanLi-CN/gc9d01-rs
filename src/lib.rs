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
#[cfg(feature = "defmt")]
use defmt;

use embedded_graphics_core::pixelcolor::{Rgb565, raw::RawU16};
use embedded_graphics_core::prelude::RawData;
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
    fn after_millis(milliseconds: u64) -> impl core::future::Future<Output = ()>;
}

pub const BUF_SIZE: usize = 24 * 48 * 2;
pub const MAX_DATA_LEN: usize = BUF_SIZE / 2;

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
            orientation: Orientation::Landscape,
            height: 160,
            width: 60,
            dx: 0,
            dy: 0,
        }
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
    buffer: &'b mut [u8],
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
    pub fn new(config: Config, bus: BUS, dc: DC, rst: RST, buffer: &'b mut [u8]) -> Self {
        Self {
            bus,
            dc,
            rst,
            config,
            buffer,
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

        self.write_command(Instruction::Cmd7E, &[0x7A]).await?; // VGL大小

        // 修改帧频
        self.write_command(
            Instruction::Cmd74,
            &[0x02, 0x0E, 0x00, 0x00, 0x28, 0x00, 0x00],
        )
        .await?;

        // 内部电压调整
        self.write_command(Instruction::Cmd98, &[0x3E]).await?;
        self.write_command(Instruction::Cmd99, &[0x3E]).await?;

        // 内部porch设置
        self.write_command(Instruction::BlankingPorchControl, &[0x0E, 0x0E])
            .await?; // 0xB5

        // gip timing start
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
                0x00, 0x00, 0x00, 0x00, 0x07, 0x01, 0x13, 0x11, 0x0B, 0x09, 0x16, 0x15, 0x1D, 0x1E,
                0x00, 0x00, 0x00, 0x00, 0x1E, 0x1D, 0x15, 0x16, 0x0A, 0x0C, 0x12, 0x14, 0x02, 0x08,
                0x00, 0x00, 0x00, 0x00,
            ],
        )
        .await?;
        // gip timing end

        // 内部电压设定开始
        self.write_command(Instruction::CmdA9, &[0x1B]).await?;
        self.write_command(Instruction::CmdA8, &[0x6B]).await?; // 第一次 A8
        self.write_command(Instruction::CmdA8, &[0x6D]).await?; // 第二次 A8
        self.write_command(Instruction::CmdA7, &[0x40]).await?;
        self.write_command(Instruction::CmdAD, &[0x47]).await?;
        self.write_command(Instruction::CmdAF, &[0x73]).await?; // 第一次 AF
        self.write_command(Instruction::CmdAF, &[0x73]).await?; // 第二次 AF
        self.write_command(Instruction::CmdAC, &[0x44]).await?;
        self.write_command(Instruction::CmdA3, &[0x6C]).await?;
        self.write_command(Instruction::CmdCB, &[0x00]).await?;
        self.write_command(Instruction::CmdCD, &[0x22]).await?;
        self.write_command(Instruction::CmdC2, &[0x10]).await?;
        self.write_command(Instruction::CmdC5, &[0x00]).await?;
        self.write_command(Instruction::CmdC6, &[0x0E]).await?;
        self.write_command(Instruction::CmdC7, &[0x1F]).await?;
        self.write_command(Instruction::CmdC8, &[0x0E]).await?;
        // 内部电压设定结束

        // Dual-Single gate select (BFh) - Set to Single gate mode (0x00) - Reverting for test
        self.write_command(Instruction::DualSingleGateSelect, &[0x00])
            .await?; // 0xBF, 选择single gate mode

        // SOU相关调整
        self.write_command(Instruction::CmdF9, &[0x20]).await?;

        // vreg电压调整
        self.write_command(Instruction::Cmd9B, &[0x3B]).await?;
        self.write_command(Instruction::Cmd93, &[0x33, 0x7F, 0x00])
            .await?;

        // VGH/VGL CLK调整 70, 71h
        self.write_command(Instruction::Cmd70, &[0x0E, 0x0F, 0x03, 0x0E, 0x0F, 0x03])
            .await?;
        self.write_command(Instruction::Cmd71, &[0x0E, 0x16, 0x03])
            .await?;

        // 内部电压调整
        self.write_command(Instruction::Cmd91, &[0x0E, 0x09])
            .await?;

        // vreg电压调整
        self.write_command(Instruction::PowerControl2, &[0x2C])
            .await?; // 0xC3
        self.write_command(Instruction::PowerControl3, &[0x1A])
            .await?; // 0xC4

        // gamma F0~F3h (注意伪代码中F0, F2, F1, F3的顺序)
        self.write_command(
            Instruction::SetGamma1,
            &[0x51, 0x13, 0x0C, 0x06, 0x00, 0x2F],
        )
        .await?; // 0xF0
        self.write_command(
            Instruction::SetGamma3,
            &[0x51, 0x13, 0x0C, 0x06, 0x00, 0x33],
        )
        .await?; // 0xF2
        self.write_command(
            Instruction::SetGamma2,
            &[0x3C, 0x94, 0x4F, 0x33, 0x34, 0xCF], // Corrected 0CF to 0xCF
        )
        .await?; // 0xF1
        self.write_command(
            Instruction::SetGamma4,
            &[0x4D, 0x94, 0x4F, 0x33, 0x34, 0xCF],
        )
        .await?; // 0xF3

        self.write_command(
            Instruction::MemoryAccessControl,
            &[self.config.orientation as u8],
        ) // Set based on config (0x60 for Landscape)
        .await?; // 0x36

        self.write_command(
            Instruction::DisplayFunctionControl,
            &[0x0A, 0x80, 0x27, 0x00],
        ) // Set Source Driver Output Scan Direction to Reverse, Gate Driver to Normal (0x80)
        .await?; // 0xB6

        self.write_command(Instruction::SleepOut, &[]).await?; // 0x11
        TIMER::after_millis(200).await;
        self.write_command(Instruction::DisplayOn, &[]).await?; // 0x29

        Ok(()) // init function ends here
    }

    pub async fn reset(&mut self) -> Result<(), Error<BusE, PinE>> {
        // 硬件复位序列: RST 拉低 -> 延时10ms -> RST 拉高 -> 延时120ms
        self.rst.set_low().map_err(Error::Pin)?;
        TIMER::after_millis(10).await;
        self.rst.set_high().map_err(Error::Pin)?;
        TIMER::after_millis(120).await;
        Ok(())
    }

    async fn write_command(
        &mut self,
        instruction: Instruction,
        params: &[u8],
    ) -> Result<(), Error<BusE, PinE>> {
        let dc_low_res = self.dc.set_low().map_err(Error::Pin);
        if dc_low_res.is_err() {
            return dc_low_res;
        }

        let cmd_bytes = [instruction as u8];
        let cmd_res = self.bus.write(&cmd_bytes).await.map_err(Error::Bus);

        if cmd_res.is_ok() && !params.is_empty() {
            let dc_high_res = self.dc.set_high().map_err(Error::Pin);
            if dc_high_res.is_err() {
                return dc_high_res;
            }
            let param_res = self.bus.write(params).await.map_err(Error::Bus);
            if param_res.is_err() {
                param_res
            } else {
                Ok(())
            }
        } else {
            if cmd_res.is_err() { cmd_res } else { Ok(()) }
        }
    }

    fn start_data_internal(&mut self) -> Result<(), PinE> {
        self.dc.set_high()
    }

    pub async fn set_address_window(
        &mut self,
        sx: u16,
        sy: u16,
        ex: u16,
        ey: u16,
    ) -> Result<(), Error<BusE, PinE>> {
        let final_sx = sx + self.config.dx;
        let final_ex = ex + self.config.dx;
        let final_sy = sy + self.config.dy;
        let final_ey = ey + self.config.dy;

        self.write_command(
            Instruction::RowAddressSet,
            &[
                (final_sx >> 8) as u8,
                final_sx as u8,
                (final_ex >> 8) as u8,
                final_ex as u8,
            ],
        )
        .await?;
        self.write_command(
            Instruction::ColumnAddressSet,
            &[
                (final_sy >> 8) as u8,
                final_sy as u8,
                (final_ey >> 8) as u8,
                final_ey as u8,
            ],
        )
        .await
    }

    pub async fn fill_color(&mut self, color: Rgb565) -> Result<(), Error<BusE, PinE>> {
        // fill_color should always fill the entire physical screen area (0,0) to (width-1, height-1)
        // set_address_window now handles the mapping to GRAM based on orientation and offsets
        self.set_address_window(0, 0, self.config.width - 1, self.config.height - 1)
            .await?;
        self.write_command(Instruction::MemoryWrite, &[]).await?;

        let internal_dc_res: Result<(), PinE> = self.start_data_internal();
        let dc_res: Result<(), Error<BusE, PinE>> = internal_dc_res.map_err(Error::Pin);
        if dc_res.is_err() {
            return dc_res;
        }

        let color_val = RawU16::from(color).into_inner();
        let high_byte = (color_val >> 8) as u8;
        let low_byte = color_val as u8;

        for chunk in self.buffer.chunks_mut(2) {
            chunk[0] = high_byte;
            chunk[1] = low_byte;
        }

        let total_pixels = self.config.width as usize * self.config.height as usize;
        let mut pixels_sent = 0;
        let mut first_bus_error: Option<BusE> = None;

        while pixels_sent < total_pixels {
            let remaining_pixels = total_pixels - pixels_sent;
            let current_chunk_pixels = core::cmp::min(remaining_pixels, MAX_DATA_LEN);
            let bytes_to_send = current_chunk_pixels * 2;
            if let Err(e) = self.bus.write(&self.buffer[..bytes_to_send]).await {
                first_bus_error = Some(e);
                break;
            }
            pixels_sent += current_chunk_pixels;
        }

        if let Some(bus_err) = first_bus_error {
            Err(Error::Bus(bus_err))
        } else {
            Ok(())
        }
    }

    pub async fn write_area(
        &mut self,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
        data: &[Rgb565],
    ) -> Result<(), Error<BusE, PinE>> {
        self.set_address_window(x, y, x + width - 1, y + height - 1)
            .await?;
        self.write_command(Instruction::MemoryWrite, &[]).await?;

        let internal_dc_res: Result<(), PinE> = self.start_data_internal();
        let dc_res: Result<(), Error<BusE, PinE>> = internal_dc_res.map_err(Error::Pin);
        if dc_res.is_err() {
            return dc_res;
        }

        let mut current_pixel_index = 0;
        let mut first_bus_error: Option<BusE> = None;
        while current_pixel_index < data.len() {
            let mut buffer_idx = 0;
            while buffer_idx < self.buffer.len() && current_pixel_index < data.len() {
                let color_val = RawU16::from(data[current_pixel_index]).into_inner();
                self.buffer[buffer_idx] = (color_val >> 8) as u8;
                self.buffer[buffer_idx + 1] = color_val as u8;
                buffer_idx += 2;
                current_pixel_index += 1;
            }
            if buffer_idx > 0 {
                if let Err(e) = self.bus.write(&self.buffer[..buffer_idx]).await {
                    first_bus_error = Some(e);
                }
            }
        }
        if let Some(bus_err) = first_bus_error {
            Err(Error::Bus(bus_err))
        } else {
            Ok(())
        }
    }
}
