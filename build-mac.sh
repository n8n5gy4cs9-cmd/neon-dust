#!/bin/sh
# Build NeonDust.app on macOS (Apple Silicon or Intel). Run on a Mac: ./build-mac.sh
set -e
cargo build --release
APP=target/NeonDust.app
rm -rf "$APP"; mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/neon-dust "$APP/Contents/MacOS/neon-dust"
cat > "$APP/Contents/Info.plist" <<PL
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>Neon Dust</string>
<key>CFBundleExecutable</key><string>neon-dust</string>
<key>CFBundleIdentifier</key><string>com.neondust.game</string>
<key>CFBundleVersion</key><string>0.1.0</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>NSHighResolutionCapable</key><true/>
<key>LSMinimumSystemVersion</key><string>11.0</string>
</dict></plist>
PL
echo "Built $APP  (run: open $APP  or  cargo run --release)"
