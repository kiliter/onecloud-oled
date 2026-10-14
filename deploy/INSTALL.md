# 玩客云 OLED v1.0.0 安装说明

这是 ARMv7 musl 静态安装包，设备无需 Rust、Cargo、Python 或编译器。服务已配置为 /dev/gpiochip0、SDA line 17、SCL line 11，沿用这台玩客云已经点亮的接线。

## 显示效果

第一页居中显示 One-KVM、当前 IPv4 和 HH:MM:SS，标题和时间为 10×20 大字，IP 为 8×13 粗体。信息每 2 秒更新；停留 6 秒后以 450 毫秒滑动转场进入 Grok Bot 风格动画，停留 4 秒后滑回第一页，持续轮播。

Bot 有眨眼、左右看、呼吸、弹动和表情变化。动画预先生成并嵌入二进制，运行时差分绘制，以单调时间推进；目标 12 fps，实际速度取决于 GPIO/I2C 传输时间。

本版本已移除原 Python 复刻模式。所有显示均通过 ssd1306 和标准 I2C，检查 ACK；初始化、清屏或写入失败时退出，systemd 每隔 5 秒重启。网络未就绪时显示 -- 并每 2 秒重试。

## 复制

在电脑上执行，将示例 IP 改为设备实际 IP；安装包文件放在当前目录：

```bash
ONECLOUD_IP=192.168.1.100
scp onecloud-oled-armv7-v1.0.0.tar.gz "root@${ONECLOUD_IP}:/opt/"
```

## 一键安装

若 OLED 程序正在前台运行，先按 Ctrl+C，然后在玩客云上以 root 执行：

```bash
cd /opt
tar -xzf onecloud-oled-armv7-v1.0.0.tar.gz
cd onecloud-oled-armv7-v1.0.0
sh install.sh
```

脚本核对所有文件的 SHA-256，再停止旧服务、安装二进制和服务文件、将 gpio.conf 更新为标准 17/11 模式，随后启用开机自启并重启。校验失败会中止安装。

## 手动安装

如需逐步执行，在解压目录以 root 运行：

```bash
sha256sum -c SHA256SUMS
systemctl stop onecloud-oled.service
install -m 0755 onecloud-oled /usr/local/bin/onecloud-oled
install -m 0644 onecloud-oled.service /etc/systemd/system/onecloud-oled.service
mkdir -p /etc/systemd/system/onecloud-oled.service.d
install -m 0644 onecloud-oled-gpio.conf /etc/systemd/system/onecloud-oled.service.d/gpio.conf
systemctl daemon-reload
systemctl enable onecloud-oled.service
systemctl restart onecloud-oled.service
systemctl status onecloud-oled.service --no-pager
```

## 查看状态与日志

```bash
/usr/local/bin/onecloud-oled --version
systemctl is-enabled onecloud-oled.service
journalctl -u onecloud-oled.service -n 30 --no-pager
```

版本应为 onecloud-oled 1.0.0，开机状态应为 enabled。时间遵循设备系统时区；需要北京时间时执行：

```bash
timedatectl set-timezone Asia/Shanghai
systemctl restart onecloud-oled.service
```

## 前台运行与诊断

```bash
systemctl stop onecloud-oled.service
./onecloud-oled --gpio /dev/gpiochip0 17 11
```

按 Ctrl+C 退出，再用 systemctl restart onecloud-oled.service 恢复服务。

```bash
# 只查询，不申请引脚。
./onecloud-oled --gpio-info /dev/gpiochip0

# 停止服务后，诊断指定两条引脚的电平与 ACK。
./onecloud-oled --gpio-probe /dev/gpiochip0 17 11
```

--gpio 的参数顺序为芯片路径、SDA line、SCL line，使用 GPIO 字符设备 line 编号。默认地址 0x3C、软件 I2C 配置频率 50000 Hz、开漏输出、保持原有上拉；支持 --address 0x3D、--frequency 1000 至 100000、--pull-up。

不带参数时依次尝试 /dev/i2c-0 和 /dev/i2c-1。当前接线使用 GPIO，应保留安装包的 gpio.conf。

## HTML 预览与验证边界

在电脑浏览器中打开 oled-preview.html，可离线比较字号和动画。Rust 已实现默认大字居中、6/4 秒及滑动效果；HTML 中手动选择其他方案不会改变设备配置。

轮播时间边界、长时间播放、差分绘制与像素擦除、最长 IPv4 居中和 Bot 帧范围通过测试；包内二进制按 ARMv7 静态目标构建。旧版标准 GPIO 17/11 模式已经由用户实机点亮；新版动画的实际流畅度、服务持续运行和开机自启需在设备上验收。
