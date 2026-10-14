use std::{env, fs, path::PathBuf, time::Duration};

use onecloud_oled::ui;

// 输出 Rust 实际使用的点阵帧，便于在没有 OLED 的电脑上核对布局。
fn main() -> std::io::Result<()> {
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
        let mut pbm = String::from("P1\n128 64\n");
        for y in 0..64 {
            for x in 0..128 {
                // PBM 的 1 为黑色，反相后显示黑底白点阵。
                pbm.push_str(if frame.pixel(x, y) { "0 " } else { "1 " });
            }
            pbm.push('\n');
        }
        fs::write(directory.join(format!("{name}.pbm")), pbm)?;
    }
    println!("已输出点阵预览：{}", directory.display());
    Ok(())
}
