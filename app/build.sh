#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")"
OUT=${1:-build}; APP=$OUT/1401.app
[ -f Resources/NullMothSafe.efi ] || { echo "STOP: Resources/NullMothSafe.efi missing (build efi-safe first)"; exit 1; }
SDK=$(xcrun --sdk macosx --show-sdk-path)
rm -rf "$APP"; mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
xcrun swiftc -O -target x86_64-apple-macos15.0 -sdk "$SDK" -framework WebKit -framework Metal -framework IOKit \
  Sources/main.swift Sources/profile.swift -o "$APP/Contents/MacOS/1401"
cp Resources/* "$APP/Contents/Resources/"
chmod 755 "$APP/Contents/Resources/nullmoth-setup.sh"
IS=$(mktemp -d)/m.iconset; mkdir -p "$IS"
for s in 16 32 128 256 512; do
  sips -z $s $s Resources/moth-mark.jpg --setProperty format png --out "$IS/icon_${s}x${s}.png" >/dev/null
  sips -z $((s*2)) $((s*2)) Resources/moth-mark.jpg --setProperty format png --out "$IS/icon_${s}x${s}@2x.png" >/dev/null
done
iconutil -c icns "$IS" -o "$APP/Contents/Resources/1401.icns"
cat > "$APP/Contents/Info.plist" <<PL
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleDevelopmentRegion</key><string>en</string>
<key>CFBundleExecutable</key><string>1401</string>
<key>CFBundleIconFile</key><string>1401</string>
<key>CFBundleIdentifier</key><string>com.nullmoth.1401</string>
<key>CFBundleName</key><string>1401</string>
<key>CFBundleDisplayName</key><string>1401</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>1.0.10</string>
<key>CFBundleVersion</key><string>11</string>
<key>LSMinimumSystemVersion</key><string>15.0</string>
<key>NSHumanReadableCopyright</key><string>© 2026 NullMoth Systems</string>
<key>NSHighResolutionCapable</key><true/>
<key>NSAppleEventsUsageDescription</key><string>1401 asks macOS for your password to install the driver, and to restart when you click Restart.</string>
</dict></plist>
PL
codesign --force --deep -s - "$APP"
echo "built $APP ($(du -sh "$APP" | cut -f1))"
