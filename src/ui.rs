use std::{convert::Infallible, time::Duration};

use embedded_graphics::{
    mono_font::{
        ascii::{FONT_10X20, FONT_8X13_BOLD},
        MonoFont, MonoTextStyle,
    },
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};

pub const FPS: u32 = 12;
const WIDTH: usize = 128;
const HEIGHT: usize = 64;
const FRAME_BYTES: usize = WIDTH * HEIGHT / 8;
const BOT_FRAMES: &[u8; 12 * FPS as usize * FRAME_BYTES] =
    include_bytes!(concat!(env!("OUT_DIR"), "/bot-frames.bin"));
const INFO_MS: u128 = 6_000;
const BOT_MS: u128 = 4_000;
const SLIDE_MS: u128 = 450;
const CYCLE_MS: u128 = INFO_MS + BOT_MS + 2 * SLIDE_MS;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Frame([u8; FRAME_BYTES]);

impl Frame {
    pub fn blank() -> Self {
        Self([0; FRAME_BYTES])
    }

    pub fn pixel(&self, x: usize, y: usize) -> bool {
        self.0[y / 8 * WIDTH + x] & (1 << (y % 8)) != 0
    }

    // 只提交变化的像素，让 SSD1306 驱动按脏区域刷新。
    pub fn changes<'a>(
        &'a self,
        previous: &'a Self,
    ) -> impl Iterator<Item = Pixel<BinaryColor>> + 'a {
        (0..WIDTH * HEIGHT).filter_map(move |i| {
            let (x, y) = (i % WIDTH, i / WIDTH);
            let on = self.pixel(x, y);
            (on != previous.pixel(x, y)).then_some(Pixel(
                Point::new(x as i32, y as i32),
                if on {
                    BinaryColor::On
                } else {
                    BinaryColor::Off
                },
            ))
        })
    }
}

impl OriginDimensions for Frame {
    fn size(&self) -> Size {
        Size::new(WIDTH as u32, HEIGHT as u32)
    }
}

impl DrawTarget for Frame {
    type Color = BinaryColor;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(point, color) in pixels {
            if point.x < 0 || point.y < 0 || point.x >= WIDTH as i32 || point.y >= HEIGHT as i32 {
                continue;
            }
            let (x, y) = (point.x as usize, point.y as usize);
            let byte = &mut self.0[y / 8 * WIDTH + x];
            let mask = 1 << (y % 8);
            if color == BinaryColor::On {
                *byte |= mask;
            } else {
                *byte &= !mask;
            }
        }
        Ok(())
    }
}

pub fn information(ip: &str, time: &str) -> Frame {
    let mut frame = Frame::blank();
    for (text, font, y) in [
        ("One-KVM", &FONT_10X20, 0),
        (ip, &FONT_8X13_BOLD, 24),
        (time, &FONT_10X20, 44),
    ] {
        centered_text(&mut frame, text, font, y);
    }
    frame
}

fn centered_text(frame: &mut Frame, text: &str, font: &MonoFont<'_>, y: i32) {
    let width = text.chars().count() as i32 * font.character_size.width as i32;
    let x = ((WIDTH as i32 - width) / 2).max(0);
    Text::with_baseline(
        text,
        Point::new(x, y),
        MonoTextStyle::new(font, BinaryColor::On),
        Baseline::Top,
    )
    .draw(frame)
    .unwrap(); // 内存画布的错误类型为 Infallible。
}

pub fn bot(elapsed: Duration) -> Frame {
    let index = (elapsed.as_nanos() * u128::from(FPS) / 1_000_000_000
        % (BOT_FRAMES.len() / FRAME_BYTES) as u128) as usize;
    let mut frame = Frame::blank();
    frame
        .0
        .copy_from_slice(&BOT_FRAMES[index * FRAME_BYTES..(index + 1) * FRAME_BYTES]);
    frame
}

#[derive(Debug, PartialEq, Eq)]
pub enum Scene {
    Information,
    ToBot(u32),
    Bot,
    ToInformation(u32),
}

pub fn scene(elapsed: Duration) -> Scene {
    let phase = elapsed.as_millis() % CYCLE_MS;
    match phase {
        p if p < INFO_MS => Scene::Information,
        p if p < INFO_MS + SLIDE_MS => Scene::ToBot(slide_offset(p - INFO_MS)),
        p if p < INFO_MS + SLIDE_MS + BOT_MS => Scene::Bot,
        p => Scene::ToInformation(slide_offset(p - INFO_MS - SLIDE_MS - BOT_MS)),
    }
}

fn slide_offset(elapsed_ms: u128) -> u32 {
    let progress = elapsed_ms as f64 / SLIDE_MS as f64;
    (WIDTH as f64 * progress * progress * (3.0 - 2.0 * progress)).round() as u32
}

pub fn slide(from: &Frame, to: &Frame, offset: u32) -> Frame {
    let offset = offset.min(WIDTH as u32) as usize;
    let mut frame = Frame::blank();
    for page in 0..HEIGHT / 8 {
        let base = page * WIDTH;
        frame.0[base..base + WIDTH - offset].copy_from_slice(&from.0[base + offset..base + WIDTH]);
        frame.0[base + WIDTH - offset..base + WIDTH].copy_from_slice(&to.0[base..base + offset]);
    }
    frame
}

pub fn render(elapsed: Duration, information: &Frame) -> Frame {
    match scene(elapsed) {
        Scene::Information => information.clone(),
        Scene::Bot => bot(elapsed),
        Scene::ToBot(offset) => slide(information, &bot(elapsed), offset),
        Scene::ToInformation(offset) => slide(&bot(elapsed), information, offset),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeline_respects_holds_transitions_and_wraps_without_accumulating_drift() {
        for (ms, expected) in [
            (0, Scene::Information),
            (5_999, Scene::Information),
            (6_000, Scene::ToBot(0)),
            (6_225, Scene::ToBot(64)),
            (6_450, Scene::Bot),
            (10_449, Scene::Bot),
            (10_450, Scene::ToInformation(0)),
            (10_675, Scene::ToInformation(64)),
            (10_900, Scene::Information),
            (10_900 * 10_000 + 6_450, Scene::Bot),
        ] {
            assert_eq!(scene(Duration::from_millis(ms)), expected);
        }
    }

    #[test]
    fn slide_and_pixel_changes_reconstruct_the_next_frame_including_erasure() {
        let from = information("255.255.255.255", "23:59:59");
        let to = bot(Duration::from_millis(7_000));
        assert_eq!(slide(&from, &to, 0), from);
        assert_eq!(slide(&from, &to, 128), to);
        assert_eq!(slide(&from, &to, 999), to);
        let mut display = from.clone();
        display.draw_iter(to.changes(&from)).unwrap();
        assert_eq!(display, to);
        assert_eq!(to.changes(&to).count(), 0);
    }

    #[test]
    fn longest_ipv4_is_centered_and_matches_the_same_text_rendered_without_clipping() {
        let ip = "255.255.255.255";
        let frame = information(ip, "23:59:59");
        let mut expected = Frame::blank();
        Text::with_baseline(
            ip,
            Point::new(4, 24),
            MonoTextStyle::new(&FONT_8X13_BOLD, BinaryColor::On),
            Baseline::Top,
        )
        .draw(&mut expected)
        .unwrap();
        for y in 24..37 {
            for x in 0..WIDTH {
                assert_eq!(frame.pixel(x, y), expected.pixel(x, y));
            }
        }
    }

    #[test]
    fn bot_frames_animate_repeat_and_stay_inside_the_display() {
        assert_ne!(bot(Duration::ZERO), bot(Duration::from_secs(3)));
        assert_ne!(bot(Duration::ZERO), bot(Duration::from_millis(500)));
        assert_eq!(bot(Duration::ZERO), bot(Duration::from_secs(12)));
        for i in 0..12 * FPS {
            let frame = bot(Duration::from_nanos(
                u64::from(i) * 1_000_000_000 / u64::from(FPS),
            ));
            assert!(frame.0.iter().any(|byte| *byte != 0));
            for x in 0..WIDTH {
                assert!(!frame.pixel(x, 0) && !frame.pixel(x, HEIGHT - 1));
            }
            for y in 0..HEIGHT {
                assert!(!frame.pixel(0, y) && !frame.pixel(WIDTH - 1, y));
            }
        }
    }
}
