#!/bin/bash
# 构建 release 二进制并打包成 SwiftGlance.app（纯菜单栏应用）。
set -e
cd "$(dirname "$0")"

APP_NAME="SwiftGlance"
BUILD_DIR=".build/release"
APP="${APP_NAME}.app"

echo "==> swift build -c release"
swift build -c release

echo "==> 组装 ${APP}"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS"
cp "$BUILD_DIR/$APP_NAME" "$APP/Contents/MacOS/$APP_NAME"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleDisplayName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleIdentifier</key>
    <string>com.swiftglance.menubar</string>
    <key>CFBundleVersion</key>
    <string>1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>CFBundleExecutable</key>
    <string>${APP_NAME}</string>
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

echo "==> 完成: $(pwd)/$APP"
du -sh "$APP" | awk '{print "    bundle size:", $1}'
