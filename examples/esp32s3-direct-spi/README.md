# ESP32-S3 直驱 SPI 示例（GC9D01）

本示例展示如何在 ESP32‑S3 上“直接”通过 SPI 驱动 GC9D01 液晶面板，不依赖 `display-interface` 或封装驱动，便于调试底层命令与波形。示例移植自本仓库 `examples/stm32g4-direct-spi` 的初始化与渲染逻辑，并采用参考固件项目 `iso-usb-hub_v2` 中相同的引脚分配。

- 位置：`examples/esp32s3-direct-spi/`
- 逻辑分辨率：160 × 40（示例只在面板 360×360 GRAM 的一块区域渲染）
- SPI：SPI2，Mode0，默认 10 MHz

## 硬件连接（与 iso-usb-hub_v2 一致）

| 显示屏引脚 | 说明         | ESP32‑S3 引脚 |
|------------|--------------|---------------|
| SCLK/CLK   | SPI 时钟     | `GPIO12`      |
| MOSI/SDA   | SPI 主出     | `GPIO11`      |
| CS         | 片选（低有效）| `GPIO13`      |
| DC         | 数据/命令    | `GPIO10`      |
| RST        | 复位（低有效）| `GPIO14`      |
| BLK/LED    | 背光（高点亮）| `GPIO15`      |
| VCC        | 供电         | 3.3 V         |
| GND        | 地           | GND           |

注意：

- 面板与背光通常工作在 3.0–3.3 V。确保背光电流不超过板卡驱动能力，必要时串联限流或采用外部驱动。
- 接线后请确认 CS/DC/RST 的电平兼容且无短路，SPI 采用 Mode0。

## 环境准备

本示例自带 `rust-toolchain.toml`，会自动使用 `esp` 工具链通道，并声明目标 `xtensa-esp32s3-none-elf` 与组件 `rust-src`。首次构建时 rustup 会自动安装所需工具链/目标。

仍需手动安装的工具：

```bash
cargo install espflash
```

## 构建与烧录

本示例目录已提供 `.cargo/config.toml`：

- 目标：`xtensa-esp32s3-none-elf`
- Runner：`espflash flash --monitor`
- `DEFMT_LOG=info`

快速运行：

```bash
cd examples/esp32s3-direct-spi
cargo run --release
```

如需仅编译：

```bash
cargo build --release
```

## 运行现象与验证

- 串口日志包含：
  - `Starting GC9D01 Direct SPI Test Firmware (ESP32-S3)`
  - `init.time: embassy-timer=ok`
  - 初始化命令发送后的信息日志与最终 `Rendering tests completed. Example will idle.`
- 屏幕显示：
  - 先全屏（360×360 GRAM）清为黑色；
  - 然后在 160×40 的区域内，顶部一行绘制 8 个 20×20 彩色方块（品红、红、黄、绿、青、蓝、紫、白），底部一行绘制 8 个 20×20 灰阶方块（黑→白）。

## 代码结构与可调项

- 主程序：`examples/esp32s3-direct-spi/src/main.rs`
  - SPI 配置（频率/模式/管脚）：在创建 `Spi::new(...).with_sck(GPIO12).with_mosi(GPIO11)` 处，默认 `10_000_000` Hz 与 `Mode0`。
  - 引脚：`CS=GPIO13`、`DC=GPIO10`、`RST=GPIO14`、`BLK=GPIO15`（上电即点亮背光）。
  - 初始化序列：与 `stm32g4-direct-spi` 保持一致，便于对照。
  - 批量填充：`fill_area_with_color` 使用 512 字节批发送（256 像素/批）。
  - 逻辑分辨率：示例固定渲染 160×40；若要修改，可在渲染循环中调整坐标或块大小（例如把每块由 20×20 改为其它尺寸）。

> 若出现闪烁、噪点或绘制异常：
>
> - 降低 SPI 频率（如 5 MHz）。
> - 确认 DC/CS/RST 以及背光电平是否符合预期。
> - 核对面板接口（有些板子标记为 SDA/SCL，其实用于 3/4 线 SPI）。

## 许可与致谢

- 许可：继承仓库（MIT/Apache‑2.0）。
- 致谢：
  - 初始化与直驱思路来自 `examples/stm32g4-direct-spi`；
  - 引脚分配参考 `/Users/ivan/Projects/Ivan/iso-usb-hub_v2/src/main.rs` 注释与实现。
