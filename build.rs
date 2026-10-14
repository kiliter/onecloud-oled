use std::{env, fs, path::PathBuf};

// 编译时生成点阵帧，ARMv7 运行时无需逐像素计算三角函数和浮点幂。
fn main() {
    const FPS: usize = 12;
    let mut frames = Vec::with_capacity(12 * FPS * 1024);
    for frame in 0..12 * FPS {
        frames.extend(bot_frame(frame as f64 / FPS as f64));
    }
    let path = PathBuf::from(env::var_os("OUT_DIR").expect("缺少构建输出目录"));
    fs::write(path.join("bot-frames.bin"), frames).expect("写入 Bot 动画帧失败");
    println!("cargo:rerun-if-changed=build.rs");
}

fn bot_frame(seconds: f64) -> [u8; 1024] {
    let expression = (seconds / 3.0) as usize % 4;
    let pulse = (seconds * 2.4).sin();
    let bob = (seconds * 1.8).sin();
    let kick = bob.max(0.0).powi(8);
    let cx = 64.0 + (seconds * 0.85).sin() * 1.5;
    let cy = 32.0 + bob * 1.1 - kick * 1.7;
    let rx = 24.5 * (1.0 + 0.035 * pulse - 0.035 * kick);
    let ry = 22.5 * (1.0 - 0.03 * pulse + 0.05 * kick);
    let exponent = 2.25 + 0.85 * (1.0 + (seconds * 0.65).sin()) / 2.0;
    let (si, co) = (0.12 * (seconds * 1.05).sin() - 0.05).sin_cos();
    let gaze = if expression == 2 {
        (seconds * 1.1).sin() * 3.0
    } else {
        (seconds * 0.9).sin() * 1.4
    };
    let gaze_y = if expression == 2 {
        -1.7
    } else {
        (seconds * 0.7).sin() * 0.6
    };
    let blink = ((seconds % 4.7 - 3.65).abs() / 0.13).clamp(0.08, 1.0);
    let (eye_si, eye_co) = (0.26_f64 + if expression == 2 { 0.1 } else { 0.0 }).sin_cos();
    let eye = |u: f64, v: f64, x: f64, y: f64, right: bool| {
        let dx = u - x;
        let dy = v - y;
        let ex = dx * eye_co + dy * eye_si;
        let ey = -dx * eye_si + dy * eye_co;
        if expression == 1 {
            return ex.abs() <= 4.2 && (ey - (-1.8 + ex * ex * 0.14)).abs() < 1.1;
        }
        let hw: f64 = if expression == 3 { 4.0 } else { 2.8 };
        let hh = if expression == 3 {
            6.7
        } else if right {
            6.2
        } else {
            5.1
        } * blink;
        let radius = hw.min(hh);
        let qx = (ex.abs() - hw + radius).max(0.0);
        let qy = (ey.abs() - hh + radius).max(0.0);
        qx * qx + qy * qy <= radius * radius
    };
    let mut frame = [0; 1024];
    for y in 0..64 {
        for x in 0..128 {
            let dx = x as f64 + 0.5 - cx;
            let dy = y as f64 + 0.5 - cy;
            let u = dx * co + dy * si;
            let v = -dx * si + dy * co;
            let angle = v.atan2(u);
            let edge = 1.0 + 0.035 * (angle * 3.0 + seconds * 0.5).sin() + 0.025 * angle.cos();
            let inside = (u / rx).abs().powf(exponent) + (v / ry).abs().powf(exponent) <= edge;
            if inside
                && !eye(u, v, -8.0 + gaze, -5.0 + gaze_y, false)
                && !eye(u, v, 6.0 + gaze, -7.0 + gaze_y, true)
            {
                frame[y / 8 * 128 + x] |= 1 << (y % 8);
            }
        }
    }
    frame
}
