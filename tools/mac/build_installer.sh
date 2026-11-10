#!/bin/sh
# Builds Mimi and makes her installer: a .dmg with the app and a shortcut to Applications.
# It uses no Finder windows, so it can run without touching the desktop.
# Usage, from the project folder: sh tools/mac/build_installer.sh
set -eu
cd "$(dirname "$0")/../.."
. tools/mac/env.sh

(cd mimi && npx tauri build --bundles app)

app="$CARGO_TARGET_DIR/release/bundle/macos/Mimi.app"
version=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$app/Contents/Info.plist")
folder="$CARGO_TARGET_DIR/release/bundle/dmg"
installer="$folder/Mimi_${version}_aarch64.dmg"

stage=$(mktemp -d)
trap 'rm -rf "${stage:?}"' EXIT
ditto "$app" "$stage/Mimi.app"
ln -s /Applications "$stage/Applications"

mkdir -p "$folder"
hdiutil create -quiet -ov -volname "Mimi" -srcfolder "$stage" -format UDZO "$installer"
if [ -n "${APPLE_SIGNING_IDENTITY:-}" ]; then
  codesign --force --sign "$APPLE_SIGNING_IDENTITY" "$installer"
fi
hdiutil verify -quiet "$installer"
echo "$installer"
