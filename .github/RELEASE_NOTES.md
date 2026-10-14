玩客云 SSD1306 OLED Rust 显示程序，128×64、地址 0x3C。

[下载安装包](https://github.com/kiliter/onecloud-oled/releases/download/v__VERSION__/onecloud-oled-armv7-v__VERSION__.tar.gz) · [SHA-256 校验文件](https://github.com/kiliter/onecloud-oled/releases/download/v__VERSION__/onecloud-oled-armv7-v__VERSION__.tar.gz.sha256)

![大字信息与 Bot 动画模拟效果](https://raw.githubusercontent.com/kiliter/onecloud-oled/__COMMIT__/docs/images/oled-cover.png)

### 效果预览

![两页轮播动图](https://raw.githubusercontent.com/kiliter/onecloud-oled/__COMMIT__/docs/images/oled-carousel.gif)

以上为 Rust 绘制函数导出的模拟效果，实际动画速度以设备为准。

### 功能

- 居中大字显示 One-KVM、IPv4 和本地时间，信息每 2 秒更新。
- 信息页 6 秒、Bot 动画页 4 秒，带滑动转场；Bot 支持眨眼、观察、弹动和表情变化。
- 预生成动画帧、差分绘制、单调时间播放，传输较慢时跳帧。
- 使用标准 I2C 并检查 ACK，初始化或刷新失败时退出，由 systemd 自动重启。
- 已配置这台玩客云的 GPIO：/dev/gpiochip0，SDA line 17、SCL line 11。
- ARMv7 musl 静态二进制，设备无需 Rust、Cargo 或 Python。

### 安装

将安装包上传到玩客云 /opt，退出前台旧程序后，以 root 执行：

```bash
cd /opt
tar -xzf onecloud-oled-armv7-v__VERSION__.tar.gz
cd onecloud-oled-armv7-v__VERSION__
sh install.sh
```

安装脚本校验文件后更新程序、重启服务并启用开机自启。

Actions 已检查格式、Clippy、单元测试、归档校验和 ARMv7 模拟执行。实际动画流畅度、服务持续运行与开机启动仍需设备验收；GPIO 17/11 模式此前已经实机点亮。
