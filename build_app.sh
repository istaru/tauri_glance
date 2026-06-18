#!/bin/bash
# 构建 release 二进制并打包成 SwiftGlance.app（纯菜单栏应用）。
# 用法：bash build_app.sh [版本号]   例：bash build_app.sh 1.2.0
set -e
cd "$(dirname "$0")"

# APP_NAME：可执行名 / 内部标识（保持 ASCII，稳定，勿改）
# DISPLAY_NAME：对外显示的中文名（访达 / 启动台 / 关于面板）
APP_NAME="SwiftGlance"
DISPLAY_NAME="看一眼"
APP="${DISPLAY_NAME}.app"
VERSION="${1:-1.0.0}"

echo "==> swift build -c release (arm64 + x86_64)"
swift build -c release --arch arm64
swift build -c release --arch x86_64

echo "==> lipo: 合并为 Universal Binary"
lipo -create \
  ".build/arm64-apple-macosx/release/${APP_NAME}" \
  ".build/x86_64-apple-macosx/release/${APP_NAME}" \
  -output ".build/release-universal-${APP_NAME}"

echo "==> 组装 ${APP}"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS"
mkdir -p "$APP/Contents/Resources"
cp ".build/release-universal-${APP_NAME}" "$APP/Contents/MacOS/$APP_NAME"
cp "Resources/AppIcon.icns" "$APP/Contents/Resources/AppIcon.icns"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>${DISPLAY_NAME}</string>
    <key>CFBundleDisplayName</key>
    <string>${DISPLAY_NAME}</string>
    <key>CFBundleIdentifier</key>
    <string>com.swiftglance.menubar</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundleExecutable</key>
    <string>${APP_NAME}</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundleIconName</key>
    <string>AppIcon</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>LSMinimumSystemVersion</key>
    <string>12.0</string>
    <key>LSUIElement</key>
    <true/>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
PLIST

# 临时签名（本机运行需要）。
codesign --force --deep --sign - "$APP" 2>/dev/null || true

echo "==> 完成: $(pwd)/$APP  (版本 ${VERSION})"
du -sh "$APP" | awk '{print "    bundle size:", $1}'
