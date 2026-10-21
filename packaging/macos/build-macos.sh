#!/usr/bin/env bash
# Builds a self-contained, drag-to-Applications .dmg of PQ Studio, a graphical
# interface for the USGS PHREEQC engine. Run on a Mac from anywhere:
#
#     packaging/macos/build-macos.sh
#
# Needs: Xcode Command Line Tools (`xcode-select --install`), CMake
# (`brew install cmake`), and Rust via rustup (https://rustup.rs).
#
# What it does: downloads the official PHREEQC source release, verifies its
# checksum, builds the `phreeqc` engine as a universal (arm64 + x86_64)
# binary, builds the app for both architectures and merges them with `lipo`,
# bundles engine + databases + notices into the .app, signs it, and wraps it
# in a .dmg with an Applications shortcut.
#
# Optional environment variables (all skipped when unset):
#   MACOS_SIGN_IDENTITY   e.g. "Developer ID Application: Name (TEAMID)".
#                         Unset = ad-hoc signature (enough for Apple silicon
#                         to run it; users still see the first-open prompt).
#   NOTARY_PROFILE        a `xcrun notarytool store-credentials` profile name.
#                         Requires MACOS_SIGN_IDENTITY. Notarizes and staples.
#   PHREEQC_TARBALL       path to an already-downloaded source tarball.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck source=../phreeqc.env
source "$ROOT/packaging/phreeqc.env"
PHREEQC_URL="https://github.com/phreeqc-dev/phreeqc3/releases/download/${PHREEQC_TAG}/phreeqc-${PHREEQC_VERSION}.tar.gz"

APP_ID="no.laerk.PQStudio"
APP_BUNDLE_NAME="PQ Studio"
APP_DISPLAY_NAME="PQ Studio"
EXE_NAME="pq-studio"
MIN_MACOS="11.0"

WORK="$ROOT/packaging/macos/build"
OUT="$ROOT/dist/out"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)"

[[ "$(uname -s)" == "Darwin" ]] || { echo "error: this script must run on macOS" >&2; exit 1; }
for tool in cmake cargo rustup lipo codesign hdiutil iconutil shasum curl; do
    command -v "$tool" >/dev/null || { echo "error: '$tool' not found (see header for requirements)" >&2; exit 1; }
done

rm -rf "$WORK"
mkdir -p "$WORK" "$OUT"
export MACOSX_DEPLOYMENT_TARGET="$MIN_MACOS"

echo "==> PHREEQC engine ${PHREEQC_VERSION}"
tarball="${PHREEQC_TARBALL:-$WORK/phreeqc-${PHREEQC_VERSION}.tar.gz}"
if [[ ! -f "$tarball" ]]; then
    curl -fL --retry 3 -o "$tarball" "$PHREEQC_URL"
fi
echo "${PHREEQC_SHA256}  ${tarball}" | shasum -a 256 -c -
tar -xzf "$tarball" -C "$WORK"
src="$WORK/phreeqc-${PHREEQC_VERSION}"

# Unmodified upstream source; build configuration only (see dist/NOTICE.md).
cmake -S "$src" -B "$WORK/phreeqc-build" \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_OSX_ARCHITECTURES="arm64;x86_64" \
    -DCMAKE_OSX_DEPLOYMENT_TARGET="$MIN_MACOS" \
    -DPHRQC_USE_GMP=OFF
cmake --build "$WORK/phreeqc-build" --target phreeqc --config Release -j "$(sysctl -n hw.ncpu)"
engine="$(find "$WORK/phreeqc-build" -name phreeqc -type f -perm -u+x | head -1)"
[[ -n "$engine" ]] || { echo "error: phreeqc engine binary not produced" >&2; exit 1; }
lipo "$engine" -verify_arch arm64 x86_64

echo "==> engine smoke test"
smoke="$WORK/smoke"
mkdir -p "$smoke"
printf 'DATABASE %s\nSOLUTION 1\n    pH 7\n    Na 1\n    Cl 1\nEND\n' "$src/database/phreeqc.dat" > "$smoke/in.pqi"
(cd "$smoke" && "$engine" in.pqi out.pqo < /dev/null >/dev/null)
grep -q "End of simulation" "$smoke/out.pqo" || { echo "error: engine smoke test failed" >&2; exit 1; }

echo "==> app (arm64 + x86_64)"
rustup target add aarch64-apple-darwin x86_64-apple-darwin
for target in aarch64-apple-darwin x86_64-apple-darwin; do
    cargo build --manifest-path "$ROOT/Cargo.toml" --release --target "$target"
done
universal_exe="$WORK/$EXE_NAME"
lipo -create -output "$universal_exe" \
    "$ROOT/target/aarch64-apple-darwin/release/pq-studio" \
    "$ROOT/target/x86_64-apple-darwin/release/pq-studio"
lipo "$universal_exe" -verify_arch arm64 x86_64

echo "==> bundle"
app="$WORK/$APP_BUNDLE_NAME.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources/database" "$app/Contents/Resources/licenses"
install -m 755 "$universal_exe" "$app/Contents/MacOS/$EXE_NAME"
# The app looks for the engine next to its own executable first.
install -m 755 "$engine" "$app/Contents/MacOS/phreeqc"
install -m 644 "$src"/database/*.dat "$app/Contents/Resources/database/"
for f in README.md LICENSE NOTICE.md THIRD_PARTY_LICENSES.md FONT_LICENSE.txt; do
    install -m 644 "$ROOT/dist/$f" "$app/Contents/Resources/licenses/$f"
done
install -m 644 "$src/doc/NOTICE" "$app/Contents/Resources/licenses/PHREEQC_NOTICE.txt"

iconset="$WORK/AppIcon.iconset"
mkdir -p "$iconset"
for s in 16 32 128 256 512; do
    cp "$ROOT/assets/icon/pq-studio-$s.png" "$iconset/icon_${s}x${s}.png"
    cp "$ROOT/assets/icon/pq-studio-$((s * 2)).png" "$iconset/icon_${s}x${s}@2x.png"
done
iconutil -c icns -o "$app/Contents/Resources/AppIcon.icns" "$iconset"

cat > "$app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key><string>en</string>
    <key>CFBundleExecutable</key><string>${EXE_NAME}</string>
    <key>CFBundleIdentifier</key><string>${APP_ID}</string>
    <key>CFBundleName</key><string>PQ Studio</string>
    <key>CFBundleDisplayName</key><string>${APP_DISPLAY_NAME}</string>
    <key>CFBundleIconFile</key><string>AppIcon</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
    <key>CFBundleShortVersionString</key><string>${VERSION}</string>
    <key>CFBundleVersion</key><string>${VERSION}</string>
    <key>LSMinimumSystemVersion</key><string>${MIN_MACOS}</string>
    <key>LSApplicationCategoryType</key><string>public.app-category.education</string>
    <key>NSHighResolutionCapable</key><true/>
    <key>NSHumanReadableCopyright</key><string>Not affiliated with or endorsed by the USGS. Includes the USGS PHREEQC engine.</string>
</dict>
</plist>
EOF
plutil -lint "$app/Contents/Info.plist"

echo "==> sign"
if [[ -n "${MACOS_SIGN_IDENTITY:-}" ]]; then
    # Inner executables first, then the bundle (Apple's recommended order).
    codesign --force --options runtime --timestamp -s "$MACOS_SIGN_IDENTITY" "$app/Contents/MacOS/phreeqc"
    codesign --force --options runtime --timestamp -s "$MACOS_SIGN_IDENTITY" "$app"
else
    codesign --force --deep -s - "$app"
fi
codesign --verify --deep --strict --verbose=2 "$app"

echo "==> dmg"
stage="$WORK/dmg"
mkdir -p "$stage"
cp -R "$app" "$stage/"
ln -s /Applications "$stage/Applications"
cp "$ROOT/dist/README.md" "$stage/README.md"
cp "$ROOT/dist/NOTICE.md" "$stage/NOTICE.md"
dmg="$OUT/PQ-Studio-${VERSION}-macos-universal.dmg"
rm -f "$dmg"
hdiutil create -volname "$APP_BUNDLE_NAME" -srcfolder "$stage" -ov -format UDZO "$dmg"

if [[ -n "${MACOS_SIGN_IDENTITY:-}" ]]; then
    codesign --force --timestamp -s "$MACOS_SIGN_IDENTITY" "$dmg"
    if [[ -n "${NOTARY_PROFILE:-}" ]]; then
        xcrun notarytool submit "$dmg" --keychain-profile "$NOTARY_PROFILE" --wait
        xcrun stapler staple "$dmg"
    fi
elif [[ -n "${NOTARY_PROFILE:-}" ]]; then
    echo "warning: NOTARY_PROFILE ignored; notarization requires MACOS_SIGN_IDENTITY" >&2
fi

echo
echo "Built: $dmg"
