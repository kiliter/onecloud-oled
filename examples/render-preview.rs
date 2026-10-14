use std::{env, fs, path::PathBuf, time::Duration};

use onecloud_oled::ui;

// 输出 Rust 实际使用的点阵帧，便于在没有 OLED 的电脑上核对布局。
fn main() -> std::io::Result<()> {
    let animation = env::args().any(|arg| arg == "--animation");
    let directory = PathBuf::from(
        env::args_os()
            .nth(1)
            .unwrap_or_else(|| "target/preview".into()),
    );
    fs::create_dir_all(&directory)?;
    let information = ui::information("255.255.255.255", "23:59:59");
    for (name, frame) in [
        ("information", information.clone()),
        ("bot-idle", ui::bot(Duration::ZERO)),
        ("bot-happy", ui::bot(Duration::from_secs(3))),
        ("bot-thinking", ui::bot(Duration::from_secs(6))),
        ("bot-surprised", ui::bot(Duration::from_secs(9))),
        (
            "transition",
            ui::render(Duration::from_millis(6_225), &information),
        ),
    ] {
        write_frame(&directory.join(format!("{name}.pbm")), &frame)?;
    }
    if animation {
        let mut index = 0_u64;
        let mut entered_bot = false;
        loop {
            let elapsed =
                Duration::from_nanos((index * 1_000_000_000).div_ceil(u64::from(ui::FPS)));
            let scene = ui::scene(elapsed);
            if entered_bot && matches!(scene, ui::Scene::Information) {
                break;
            }
            entered_bot |= matches!(scene, ui::Scene::Bot);
            // 确定性的示例时间，每 2 秒更新，与运行中的信息刷新保持一致。
            let seconds = 12 * 3600 + 34 * 60 + 56 + elapsed.as_secs() / 2 * 2;
            let time = format!(
                "{:02}:{:02}:{:02}",
                seconds / 3600,
                seconds / 60 % 60,
                seconds % 60
            );
            let information = ui::information("192.168.1.100", &time);
            write_frame(
                &directory.join(format!("frame-{index:03}.pbm")),
                &ui::render(elapsed, &information),
            )?;
            index += 1;
        }
        fs::write(
            directory.join("animation.json"),
            format!("{{\"fps\":{},\"frames\":{index}}}\n", ui::FPS),
        )?;
    }
    println!("已输出点阵预览：{}", directory.display());
    Ok(())
}

fn write_frame(path: &std::path::Path, frame: &ui::Frame) -> std::io::Result<()> {
    let mut pbm = String::from("P1\n128 64\n");
    for y in 0..64 {
        for x in 0..128 {
            // PBM 的 1 为黑色，反相后显示黑底白点阵。
            pbm.push_str(if frame.pixel(x, y) { "0 " } else { "1 " });
        }
        pbm.push('\n');
    }
    fs::write(path, pbm)
}
