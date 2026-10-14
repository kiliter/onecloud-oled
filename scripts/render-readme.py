#!/usr/bin/env python3
"""将 Rust 导出的 PBM 点阵帧排版为 README 封面和轮播 GIF。"""

import argparse
import json
import os
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
BACKGROUND = "#edf3f6"
INK = "#1d3443"
MUTED = "#617785"
BOARD = "#355a69"


def font(size, mono=False):
    choices = (
        ["/System/Library/Fonts/Menlo.ttc", "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"]
        if mono else [os.environ.get("ONECLOUD_OLED_FONT", ""),
                      "/System/Library/Fonts/STHeiti Medium.ttc",
                      "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc"]
    )
    for candidate in choices:
        if candidate and Path(candidate).is_file():
            return ImageFont.truetype(candidate, size)
    raise RuntimeError("找不到字体；请通过 ONECLOUD_OLED_FONT 指定中文字体路径。")


def device(canvas, pixels, x, y, scale):
    draw = ImageDraw.Draw(canvas)
    width, height = 128 * scale + 60, 64 * scale + 80
    draw.rounded_rectangle((x + 5, y + 8, x + width + 5, y + height + 8), 14, fill="#d3dfe5")
    draw.rounded_rectangle((x, y, x + width, y + height), 12, fill=BOARD, outline="#284551", width=2)
    for corner_x in (x + 15, x + width - 15):
        draw.ellipse((corner_x - 5, y + 9, corner_x + 5, y + 19), fill="#f2f6f8", outline="#adbdc5", width=2)
    draw.rectangle((x + 24, y + 27, x + width - 24, y + height - 39), fill="#14252d")
    canvas.paste(pixels.convert("RGB").resize((128 * scale, 64 * scale), Image.Resampling.NEAREST), (x + 30, y + 33))
    draw.text((x + 30, y + height - 28), "ONE-KVM", fill="#dce8ed", font=font(12, True))
    draw.text((x + width - 111, y + height - 28), "128 x 64", fill="#dce8ed", font=font(12, True))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("frames", nargs="?", default="target/readme-frames", help="Rust 导出的帧目录")
    parser.add_argument("--output", default="docs/images", help="输出图片目录")
    args = parser.parse_args()
    source, output = ROOT / args.frames, ROOT / args.output
    output.mkdir(parents=True, exist_ok=True)
    metadata = json.loads((source / "animation.json").read_text())
    frames = [Image.open(source / f"frame-{i:03}.pbm").copy() for i in range(metadata["frames"])]
    assert frames and all(frame.size == (128, 64) for frame in frames)

    cover = Image.new("RGB", (1100, 540), BACKGROUND)
    draw = ImageDraw.Draw(cover)
    draw.text((48, 31), "onecloud-oled", fill=INK, font=font(43, True))
    draw.text((49, 96), "玩客云上的大字信息与像素动画", fill=MUTED, font=font(23))
    device(cover, frames[0], 70, 164, 3)
    device(cover, frames[84], 586, 164, 3)
    draw.text((71, 461), "信息页 · 大字居中 · 6 秒", fill=INK, font=font(21))
    draw.text((588, 461), "Bot 动画 · 表情变化 · 4 秒", fill=INK, font=font(21))
    draw.text((49, 508), "SSD1306 / 0.96 寸 / 标准 I2C / ARMv7 静态安装包", fill=MUTED, font=font(15))
    cover.save(output / "oled-cover.png", optimize=True)

    pictures = []
    for pixels in frames:
        picture = Image.new("RGB", (704, 466), BACKGROUND)
        draw = ImageDraw.Draw(picture)
        draw.text((65, 24), "信息 6 秒 → 滑动 → Bot 4 秒 → 循环", fill=INK, font=font(24))
        device(picture, pixels, 66, 79, 4)
        draw.text((66, 427), "Rust 实际点阵渲染 · 128 × 64 · 每格一个像素", fill=MUTED, font=font(18))
        pictures.append(picture)
    palette = pictures[0].quantize(colors=64, method=Image.Quantize.MEDIANCUT)
    indexed = [picture.quantize(palette=palette, dither=Image.Dither.NONE) for picture in pictures]
    # GIF 使用 10 毫秒刻度；交替 80/90 毫秒，保持约 12 fps。
    durations = [(round((i + 1) * 100 / metadata["fps"]) - round(i * 100 / metadata["fps"])) * 10
                 for i in range(len(indexed))]
    indexed[0].save(output / "oled-carousel.gif", save_all=True, append_images=indexed[1:],
                    loop=0, duration=durations, disposal=2, optimize=False)
    print(f"已生成封面与轮播动图：{output}")


if __name__ == "__main__":
    main()
