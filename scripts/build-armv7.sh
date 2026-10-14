#!/bin/sh
set -eu

project_dir=$(cd "$(dirname "$0")/.." && pwd)
cd "$project_dir"

# 在 Docker 中安装交叉编译目标，不修改电脑上的 Rust 工具链。
docker run --rm \
    --mount "type=bind,src=$project_dir,dst=/work" \
    --workdir /work \
    --env CARGO_HOME=/work/target/cargo-home \
    rust:1.90-bookworm sh -eu -c '
        rustup target add armv7-unknown-linux-musleabihf
        rust_host=$(rustc -vV | sed -n "s/^host: //p")
        rust_sysroot=$(rustc --print sysroot)
        export CARGO_TARGET_ARMV7_UNKNOWN_LINUX_MUSLEABIHF_LINKER="$rust_sysroot/lib/rustlib/$rust_host/bin/rust-lld"
        cargo build --release --locked --target armv7-unknown-linux-musleabihf
    '

version=$(awk '/^\[package\]/{in_package=1;next} /^\[/{in_package=0} in_package && /^version = /{gsub(/"/, "", $3);print $3;exit}' Cargo.toml)
test -n "$version"
package_name="onecloud-oled-armv7-v$version"
package_dir="$project_dir/dist/$package_name"
mkdir -p "$package_dir"
cp target/armv7-unknown-linux-musleabihf/release/onecloud-oled "$package_dir/"
chmod 0755 "$package_dir/onecloud-oled"
cp onecloud-oled.service deploy/INSTALL.md deploy/onecloud-oled-gpio.conf deploy/install.sh "$package_dir/"
chmod 0755 "$package_dir/install.sh"
cp preview/oled-preview.html "$package_dir/"

if command -v sha256sum >/dev/null 2>&1; then
    (cd "$package_dir" && sha256sum onecloud-oled onecloud-oled.service onecloud-oled-gpio.conf install.sh INSTALL.md oled-preview.html > SHA256SUMS)
else
    (cd "$package_dir" && shasum -a 256 onecloud-oled onecloud-oled.service onecloud-oled-gpio.conf install.sh INSTALL.md oled-preview.html > SHA256SUMS)
fi

tar -czf "dist/$package_name.tar.gz" -C dist "$package_name"
if command -v sha256sum >/dev/null 2>&1; then
    (cd dist && sha256sum "$package_name.tar.gz" > "$package_name.tar.gz.sha256")
else
    (cd dist && shasum -a 256 "$package_name.tar.gz" > "$package_name.tar.gz.sha256")
fi
printf '安装包已生成：%s/dist/%s.tar.gz\n' "$project_dir" "$package_name"
