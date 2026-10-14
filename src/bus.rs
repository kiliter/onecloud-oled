use std::time::{Duration, Instant};

use bitbang_hal_ng::i2c::{Error as BitbangError, I2cBB};
use embedded_hal::{
    delay::DelayNs,
    digital::{InputPin, OutputPin},
    i2c::{Error, ErrorKind, ErrorType, I2c, Operation},
};
use linux_embedded_hal::{
    gpio_cdev::{Chip, LineRequestFlags},
    CdevPin, CdevPinError, I2CError, I2cdev,
};

pub struct SpinDelay;

impl DelayNs for SpinDelay {
    fn delay_ns(&mut self, ns: u32) {
        // Linux sleep 的调度粒度不适合 I2C 的微秒时序。
        let start = Instant::now();
        let duration = Duration::from_nanos(u64::from(ns));
        while start.elapsed() < duration {
            std::hint::spin_loop();
        }
    }
}

type SoftwareI2c = I2cBB<CdevPin, CdevPin, SpinDelay, CdevPinError>;

pub enum Bus {
    Hardware(I2cdev),
    Gpio(SoftwareI2c),
}

#[derive(Debug)]
pub enum BusError {
    Hardware(I2CError),
    Gpio(BitbangError<CdevPinError>),
}

impl Error for BusError {
    fn kind(&self) -> ErrorKind {
        match self {
            Self::Hardware(err) => err.kind(),
            Self::Gpio(err) => err.kind(),
        }
    }
}

impl ErrorType for Bus {
    type Error = BusError;
}

impl I2c for Bus {
    fn transaction(
        &mut self,
        address: u8,
        operations: &mut [Operation<'_>],
    ) -> Result<(), BusError> {
        let result = match self {
            Self::Hardware(i2c) => i2c
                .transaction(address, operations)
                .map_err(BusError::Hardware),
            Self::Gpio(i2c) => i2c.transaction(address, operations).map_err(BusError::Gpio),
        };
        if let Err(err) = &result {
            // display-interface 会丢弃底层错误，必须提前打印 errno 或 NoAck。
            eprintln!("I2C 地址 0x{address:02X} 通信失败：{err:?}");
        }
        result
    }
}

fn gpio_pins(
    path: &str,
    sda_line: u32,
    scl_line: u32,
    pull_up: bool,
) -> Result<(CdevPin, CdevPin), String> {
    if sda_line == scl_line {
        return Err("SDA 与 SCL 不能使用同一条 GPIO line".into());
    }

    let mut chip = Chip::new(path).map_err(|err| format!("打开 {path} 失败：{err}"))?;
    eprintln!(
        "GPIO 软件 I2C：{path}（{}），SDA line {sda_line}，SCL line {scl_line}，开漏输出，内部上拉 {}",
        chip.label(),
        if pull_up { "启用" } else { "保持原设置" }
    );

    let sda = request_pin(&mut chip, sda_line, pull_up)?;
    let scl = request_pin(&mut chip, scl_line, pull_up)?;
    Ok((sda, scl))
}

pub fn gpio_bus(
    path: &str,
    sda_line: u32,
    scl_line: u32,
    frequency: u32,
    pull_up: bool,
) -> Result<Bus, String> {
    if !(1_000..=100_000).contains(&frequency) {
        return Err("软件 I2C 频率须在 1000 至 100000 Hz 之间".into());
    }
    let (mut sda, mut scl) = gpio_pins(path, sda_line, scl_line, pull_up)?;
    // 留出屏幕上电和引脚释放后的稳定时间。
    std::thread::sleep(Duration::from_millis(200));
    for (name, pin) in [("SDA", &mut sda), ("SCL", &mut scl)] {
        let high = pin
            .is_high()
            .map_err(|err| format!("读取 {name} 电平失败：{err}"))?;
        if !high {
            return Err(format!("{name} 空闲电平为低，请核对引脚、供电及上拉"));
        }
    }

    // 使用开漏输出，并在地址和每个数据字节之后检查 ACK。
    eprintln!("软件 I2C 配置频率：{frequency} Hz（实际频率受 GPIO 调用开销影响）");
    Ok(Bus::Gpio(I2cBB::new(scl, sda, SpinDelay, frequency)))
}

fn request_pin(chip: &mut Chip, offset: u32, pull_up: bool) -> Result<CdevPin, String> {
    let line = chip
        .get_line(offset)
        .map_err(|err| format!("获取 GPIO line {offset} 失败：{err}"))?;
    let info = line
        .info()
        .map_err(|err| format!("读取 GPIO line {offset} 信息失败：{err}"))?;
    eprintln!(
        "GPIO line {offset}：名称 {}，当前使用者 {}",
        info.name().unwrap_or("未命名"),
        info.consumer().unwrap_or("无")
    );
    let mut flags = LineRequestFlags::OUTPUT | LineRequestFlags::OPEN_DRAIN;
    if pull_up {
        // gpio-cdev 0.6 未命名此标志；Linux GPIO v1 ABI 的上拉位为 1 << 5。
        flags |= LineRequestFlags::from_bits_retain(1 << 5);
    }
    let handle = line
        .request(flags, 1, "onecloud-oled")
        .map_err(|err| format!("申请 GPIO line {offset} 失败：{err}"))?;
    CdevPin::new(handle).map_err(|err| format!("初始化 GPIO line {offset} 失败：{err}"))
}

fn check_pin_levels(sda: &mut CdevPin, scl: &mut CdevPin) -> Result<(), String> {
    let check = (|| {
        scl.set_low().map_err(|err| err.to_string())?;
        sda.set_low().map_err(|err| err.to_string())?;
        SpinDelay.delay_us(200);
        for (name, pin) in [("SDA", &mut *sda), ("SCL", &mut *scl)] {
            let low = pin.is_low().map_err(|err| err.to_string())?;
            eprintln!(
                "{name} 主动拉低后读回：{}",
                if low { "低" } else { "高（异常）" }
            );
            if !low {
                return Err(format!("{name} 无法读回低电平，请检查 GPIO 驱动或外部电路"));
            }
        }
        // 在 SCL 为低时释放 SDA，避免制造额外的 START。
        for (name, pin) in [("SDA", &mut *sda), ("SCL", &mut *scl)] {
            pin.set_high().map_err(|err| err.to_string())?;
            SpinDelay.delay_us(200);
            let high = pin.is_high().map_err(|err| err.to_string())?;
            eprintln!(
                "{name} 释放 200 微秒后读回：{}",
                if high { "高" } else { "低（异常）" }
            );
            if !high {
                return Err(format!("{name} 释放后仍为低，请检查上拉或外部电路"));
            }
        }
        Ok(())
    })();
    // 检查失败时也释放引脚；句柄随后关闭。
    let release_sda = sda.set_high().map_err(|err| err.to_string());
    let release_scl = scl.set_high().map_err(|err| err.to_string());
    check.and(release_sda).and(release_scl)
}

fn probe_address(i2c: &mut SoftwareI2c, address: u8) -> Result<(), String> {
    let result = (|| {
        i2c.raw_i2c_start()
            .map_err(|err| format!("START 失败：{err:?}"))?;
        for (phase, byte) in [("地址", address << 1), ("控制", 0x00), ("NOP 命令", 0xE3)] {
            i2c.raw_write_to_slave(&[byte])
                .map_err(|err| format!("{phase}字节 0x{byte:02X} 失败：{err:?}"))?;
        }
        Ok(())
    })();
    // SSD1306 的 NOP 不改变显示内容，每次探测都发送 STOP。
    let stop = i2c
        .raw_i2c_stop()
        .map_err(|err| format!("STOP 失败：{err:?}"));
    result.and(stop)
}

pub fn gpio_probe(path: &str, first: u32, second: u32) -> Result<(), String> {
    if first == second {
        return Err("SDA 与 SCL 不能使用同一条 GPIO line".into());
    }
    let mut detected = false;
    // 仅操作用户指定的两条 line，不扫描其他引脚或其他 I2C 地址。
    // 先完成两个方向的原设置测试，再启用上拉，避免影响原设置结果。
    for pull_up in [false, true] {
        for (sda_line, scl_line) in [(first, second), (second, first)] {
            eprintln!("\n========== SDA={sda_line}，SCL={scl_line}，内部上拉={pull_up} ==========");
            let (mut sda, mut scl) = match gpio_pins(path, sda_line, scl_line, pull_up) {
                Ok(pins) => pins,
                Err(err) => {
                    eprintln!("本配置无法申请引脚：{err}");
                    continue;
                }
            };
            std::thread::sleep(Duration::from_millis(200));
            if let Err(err) = check_pin_levels(&mut sda, &mut scl) {
                eprintln!("本配置跳过地址探测：{err}");
                continue;
            }
            for frequency in [50_000, 5_000] {
                let mut i2c = I2cBB::new(scl, sda, SpinDelay, frequency);
                for address in [0x3C, 0x3D] {
                    match probe_address(&mut i2c, address) {
                        Ok(()) => {
                            detected = true;
                            eprintln!(
                                "{frequency} Hz，0x{address:02X}：地址、控制、NOP 均收到 ACK"
                            );
                            eprintln!("可试运行：./onecloud-oled --gpio {path} {sda_line} {scl_line} --address 0x{address:02X} --frequency {frequency}{}", if pull_up { " --pull-up" } else { "" });
                        }
                        Err(err) => eprintln!("{frequency} Hz，0x{address:02X}：{err}"),
                    }
                }
                (scl, sda, _) = i2c.free();
            }
        }
    }
    if detected {
        Ok(())
    } else {
        Err("诊断完成：未找到能完整确认命令的 OLED；请贴出全部诊断日志".into())
    }
}

pub fn gpio_info(path: &str) -> Result<(), String> {
    // 仅查询信息，不申请引脚，也不改变引脚方向或电平。
    let chip = Chip::new(path).map_err(|err| format!("打开 {path} 失败：{err}"))?;
    println!(
        "{path}：{}，共 {} 条 GPIO line",
        chip.label(),
        chip.num_lines()
    );
    for line in chip.lines() {
        let info = line
            .info()
            .map_err(|err| format!("读取 GPIO line 信息失败：{err}"))?;
        println!(
            "line {:2}：名称 {}，使用者 {}",
            line.offset(),
            info.name().unwrap_or("未命名"),
            info.consumer().unwrap_or("无")
        );
    }
    Ok(())
}
