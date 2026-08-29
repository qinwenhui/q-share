#!/usr/bin/env bash
# macOS 打包：
#   ./scripts/bundle-macos.sh          -> dist/q-share.app
#   ./scripts/bundle-macos.sh --dmg    -> 额外压成 dist/q-share-<版本>.dmg
#
# macOS打包成.app。
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

version="$(grep -m1 '^version' Cargo.toml | sed -E 's/.*"([^"]+)".*/\1/')"
app_name="q-share"

echo "==> 构建 release (qshare-gui)"
cargo build --release -p qshare-gui

echo "==> 组装 $app_name.app"
bundle="$root/dist/$app_name.app"
rm -rf "$bundle"
mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources"

# 填好版本号生成 Info.plist
sed -e "s/@VERSION@/$version/g" \
    -e "s/@YEAR@/$(date +%Y)/g" \
    "$root/packaging/Info.plist" > "$bundle/Contents/Info.plist"

cp "$root/target/release/qshare" "$bundle/Contents/MacOS/qshare"
cp "$root/crates/qshare-gui/assets/icon.icns" "$bundle/Contents/Resources/AppIcon.icns"

# 本地无开发者证书时做 ad-hoc 签名，保证 Apple Silicon / Gatekeeper 下可运行
if command -v codesign >/dev/null 2>&1; then
    codesign --force --deep --sign - "$bundle"
fi
echo "完成: $bundle"

if [[ "${1:-}" == "--dmg" ]]; then
    echo "==> 生成 dmg"
    dmg="$root/dist/$app_name-$version.dmg"
    rm -f "$dmg"
    hdiutil create -volname "$app_name" -srcfolder "$bundle" -ov -format UDZO "$dmg"
    echo "完成: $dmg"
fi
