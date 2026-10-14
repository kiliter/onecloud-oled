#!/bin/sh
set -eu

if [ "$(id -u)" -ne 0 ]; then
    printf '请以 root 执行安装。\n' >&2
    exit 1
fi

package_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$package_dir"
sha256sum -c SHA256SUMS

# 校验通过后再停止旧服务，更新二进制及本项目的 GPIO 配置。
if systemctl cat onecloud-oled.service >/dev/null 2>&1; then
    systemctl stop onecloud-oled.service
fi
install -m 0755 onecloud-oled /usr/local/bin/onecloud-oled
install -m 0644 onecloud-oled.service /etc/systemd/system/onecloud-oled.service
mkdir -p /etc/systemd/system/onecloud-oled.service.d
install -m 0644 onecloud-oled-gpio.conf /etc/systemd/system/onecloud-oled.service.d/gpio.conf
systemctl daemon-reload
systemctl enable onecloud-oled.service
systemctl restart onecloud-oled.service
systemctl status onecloud-oled.service --no-pager
printf '\n安装完成；查看日志：journalctl -u onecloud-oled.service -n 30 --no-pager\n'
