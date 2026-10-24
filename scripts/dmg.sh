#!/bin/sh
# Builds target/Overhead-<version>.dmg. Needs Python 3 for dmgbuild, installed into target/.
set -eu
cd "$(dirname "$0")/.."

./scripts/bundle.sh

version=$(plutil -extract CFBundleShortVersionString raw assets/Info.plist)
work=target/dmg
mkdir -p "$work"

if [ ! -x target/dmgbuild/bin/dmgbuild ]; then
    python3 -m venv target/dmgbuild
    target/dmgbuild/bin/pip install --quiet dmgbuild
fi

swift assets/dmg-background.swift "$work/background.png" 1
swift assets/dmg-background.swift "$work/background@2x.png" 2

dmg="target/Overhead-$version.dmg"
rm -f "$dmg"
target/dmgbuild/bin/dmgbuild \
    -s scripts/dmg-settings.py \
    -D app=target/Overhead.app \
    -D background="$work/background.png" \
    Overhead "$dmg"
echo "Built $dmg"
