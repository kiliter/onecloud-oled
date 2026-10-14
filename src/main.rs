mod bus;

use std::{
    thread,
    time::{Duration, Instant},
};

use bus::Bus;
use chrono::Local;
use embedded_graphics::prelude::*;
use linux_embedded_hal::I2cdev;
use local_ip_address::local_ip;
use onecloud_oled::ui;
use ssd1306::{mode::BufferedGraphicsMode, prelude::*, I2CDisplayInterface, Ssd1306};

type Oled = Ssd1306<I2CInterface<Bus>, DisplaySize128x64, BufferedGraphicsMode<DisplaySize128x64>>;

fn initialize(bus: Bus, address: u8) -> Result<Oled, String> {
    let interface = I2CDisplayInterface::new_custom_address(bus, address);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display
        .init()
        .map_err(|err| format!("OLED 初始化失败：{err:?}"))?;
    // 上电后的显存内容不确定，先完整清屏，再开始差分绘制。
    display.clear_buffer();
    display
        .flush()
        .map_err(|err| format!("OLED 初始清屏失败：{err:?}"))?;
    Ok(display)
}

fn init_display() -> Result<Oled, String> {
    for path in ["/dev/i2c-0", "/dev/i2c-1"] {
        let i2c = match I2cdev::new(path) {
            Ok(i2c) => i2c,
            Err(err) => {
                eprintln!("打开 {path} 失败：{err}");
                continue;
            }
        };

        match initialize(Bus::Hardware(i2c), 0x3C) {
            Ok(display) => {
                eprintln!("OLED 初始化成功：{path}，地址 0x3C");
                return Ok(display);
            }
            Err(err) => eprintln!("初始化 {path} 失败：{err:?}"),
        }
    }

    Err("两个 I2C 总线均无法初始化 OLED".into())
}

fn main() {
    if let Err(err) = start() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn start() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let display = match args.as_slice() {
        [] => init_display()?,
        [mode] if mode == "--version" => {
            println!("onecloud-oled {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        [mode] if mode == "--help" || mode == "-h" => {
            println!("{}", usage());
            return Ok(());
        }
        [mode, chip] if mode == "--gpio-info" => return bus::gpio_info(chip),
        [mode, chip, sda, scl] if mode == "--gpio-probe" => {
            let sda = sda.parse().map_err(|_| "SDA line 必须是非负整数")?;
            let scl = scl.parse().map_err(|_| "SCL line 必须是非负整数")?;
            return bus::gpio_probe(chip, sda, scl);
        }
        [mode, chip, sda, scl, options @ ..] if mode == "--gpio" => {
            let sda = sda.parse().map_err(|_| "SDA line 必须是非负整数")?;
            let scl = scl.parse().map_err(|_| "SCL line 必须是非负整数")?;
            let (mut address, mut frequency, mut pull_up) = (0x3C, 50_000, false);
            let mut options = options.iter();
            while let Some(option) = options.next() {
                match option.as_str() {
                    "--pull-up" => pull_up = true,
                    "--address" => {
                        let value = options.next().ok_or("--address 缺少地址")?;
                        address = u8::from_str_radix(
                            value.trim_start_matches("0x").trim_start_matches("0X"),
                            16,
                        )
                        .map_err(|_| "地址须为十六进制 0x3C 或 0x3D")?;
                        if ![0x3C, 0x3D].contains(&address) {
                            return Err("OLED 地址仅支持 0x3C 或 0x3D".into());
                        }
                    }
                    "--frequency" => {
                        frequency = options
                            .next()
                            .ok_or("--frequency 缺少频率")?
                            .parse()
                            .map_err(|_| "频率须为 1000 至 100000 Hz 的整数")?;
                    }
                    _ => return Err(format!("未知 GPIO 选项：{option}")),
                }
            }
            let display = initialize(bus::gpio_bus(chip, sda, scl, frequency, pull_up)?, address)?;
            eprintln!("OLED 初始化成功：GPIO 软件 I2C，地址 0x{address:02X}");
            display
        }
        _ => {
            return Err(usage().into());
        }
    };
    run(display)
}

fn usage() -> &'static str {
    "用法：\n  onecloud-oled                         依次尝试 /dev/i2c-0、/dev/i2c-1\n  onecloud-oled --gpio 芯片路径 SDA_line SCL_line [--address 0x3C] [--frequency 50000] [--pull-up]\n  onecloud-oled --gpio-info 芯片路径     查询 GPIO\n  onecloud-oled --gpio-probe 芯片路径 SDA_line SCL_line  诊断指定引脚\n  onecloud-oled --version\n当前玩客云：onecloud-oled --gpio /dev/gpiochip0 17 11"
}

fn run(mut display: Oled) -> Result<(), String> {
    let start = Instant::now();
    let mut next_update = start;
    let mut information = ui::Frame::blank();
    let mut previous = ui::Frame::blank();
    let mut ip_unavailable = false;
    let frame_period = Duration::from_nanos(1_000_000_000 / u64::from(ui::FPS));
    eprintln!("开始轮播：居中大字信息 6 秒 → 滑动 450 毫秒 → Bot 动画 4 秒；目标帧率 {} fps，实际取决于总线速度", ui::FPS);

    loop {
        let frame_start = Instant::now();
        if frame_start >= next_update {
            let ip = match local_ip() {
                Ok(ip) => {
                    if ip_unavailable {
                        eprintln!("IP 地址已恢复：{ip}");
                    }
                    ip_unavailable = false;
                    ip.to_string()
                }
                Err(err) => {
                    if !ip_unavailable {
                        eprintln!("获取 IP 地址失败：{err}；显示 --，每 2 秒重试");
                    }
                    ip_unavailable = true;
                    "--".into()
                }
            };
            information = ui::information(&ip, &Local::now().format("%H:%M:%S").to_string());
            next_update = Instant::now() + Duration::from_secs(2);
        }

        // 使用单调时间选择当前帧；传输变慢时跳帧，不积累播放延迟。
        let frame = ui::render(start.elapsed(), &information);
        if frame != previous {
            display
                .draw_iter(frame.changes(&previous))
                .map_err(|err| format!("绘制 OLED 失败：{err:?}"))?;
            // 总线写入失败直接退出，由 systemd 重新初始化。
            display
                .flush()
                .map_err(|err| format!("刷新 OLED 失败：{err:?}"))?;
            previous = frame;
        }
        thread::sleep(frame_period.saturating_sub(frame_start.elapsed()));
    }
}
