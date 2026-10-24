#!/bin/sh
# Builds target/Overhead.app. Pass --install to copy it to /Applications.
set -eu
cd "$(dirname "$0")/.."

cargo build --release

app=target/Overhead.app
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp target/release/overhead "$app/Contents/MacOS/Overhead"
cp assets/Info.plist "$app/Contents/"
cp assets/AppIcon.icns assets/sounds/*.caf "$app/Contents/Resources/"
codesign --force --sign - "$app"

if [ "${1:-}" = "--install" ]; then
    pkill -x Overhead 2>/dev/null || true
    rm -rf /Applications/Overhead.app
    cp -R "$app" /Applications/
    echo "Installed /Applications/Overhead.app"
else
    echo "Built $app"
fi
