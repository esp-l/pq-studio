#!/usr/bin/env bash
# Builds a self-contained Linux tarball of PQ Studio: the app, the USGS
# PHREEQC engine and its databases. Run on Linux from anywhere:
#
#     packaging/linux/build-linux.sh
#
# Needs: a C++ compiler, CMake, curl and Rust via rustup (https://rustup.rs),
# plus the usual eframe build libraries (on Debian/Ubuntu: libxkbcommon-dev
# libgtk-3-dev). For a binary that runs on older distributions, build on an
# old one (the release workflow uses Ubuntu 22.04).
#
# Writes dist/out/PQ-Studio-<version>-linux-<arch>.tar.gz. Users unpack it and
# run ./install.sh inside.
#
# Optional: PHREEQC_TARBALL = path to an already-downloaded source tarball.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck source=../phreeqc.env
source "$ROOT/packaging/phreeqc.env"
PHREEQC_URL="https://github.com/phreeqc-dev/phreeqc3/releases/download/${PHREEQC_TAG}/phreeqc-${PHREEQC_VERSION}.tar.gz"

WORK="$ROOT/packaging/linux/build"
OUT="$ROOT/dist/out"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)"
ARCH="$(uname -m)"

[[ "$(uname -s)" == "Linux" ]] || { echo "error: this script must run on Linux" >&2; exit 1; }
for tool in cmake cargo c++ curl sha256sum tar; do
    command -v "$tool" >/dev/null || { echo "error: '$tool' not found (see header for requirements)" >&2; exit 1; }
done

rm -rf "$WORK"
mkdir -p "$WORK" "$OUT"

echo "==> PHREEQC engine ${PHREEQC_VERSION}"
tarball="${PHREEQC_TARBALL:-$WORK/phreeqc-${PHREEQC_VERSION}.tar.gz}"
if [[ ! -f "$tarball" ]]; then
    curl -fL --retry 3 -o "$tarball" "$PHREEQC_URL"
fi
echo "${PHREEQC_SHA256}  ${tarball}" | sha256sum -c -
tar -xzf "$tarball" -C "$WORK"
src="$WORK/phreeqc-${PHREEQC_VERSION}"

# Unmodified upstream source; build configuration only (see dist/NOTICE.md).
# The C++ runtime is linked statically so the engine runs on distributions
# with an older libstdc++.
cmake -S "$src" -B "$WORK/phreeqc-build" \
    -DCMAKE_BUILD_TYPE=Release \
    -DPHRQC_USE_GMP=OFF \
    -DCMAKE_EXE_LINKER_FLAGS="-static-libstdc++ -static-libgcc"
cmake --build "$WORK/phreeqc-build" --target phreeqc -j "$(nproc)"
engine="$WORK/phreeqc-build/phreeqc"
[[ -x "$engine" ]] || { echo "error: phreeqc engine binary not produced" >&2; exit 1; }

echo "==> engine smoke test"
smoke="$WORK/smoke"
mkdir -p "$smoke"
printf 'DATABASE %s\nSOLUTION 1\n    pH 7\n    Na 1\n    Cl 1\nEND\n' "$src/database/phreeqc.dat" > "$smoke/in.pqi"
(cd "$smoke" && "$engine" in.pqi out.pqo < /dev/null >/dev/null 2>&1)
grep -q "End of simulation" "$smoke/out.pqo" || { echo "error: engine smoke test failed" >&2; exit 1; }

echo "==> app"
cargo build --manifest-path "$ROOT/Cargo.toml" --release

echo "==> package"
name="PQ-Studio-${VERSION}-linux-${ARCH}"
stage="$WORK/$name"
mkdir -p "$stage/database" "$stage/icons" "$stage/doc"
install -m 755 "$ROOT/target/release/pq-studio" "$stage/pq-studio"
install -m 755 "$engine" "$stage/phreeqc"
install -m 644 "$src"/database/*.dat "$stage/database/"
install -m 644 "$ROOT"/assets/icon/pq-studio-*.png "$ROOT/assets/icon/pq-studio.svg" "$stage/icons/"
for f in README.md LICENSE NOTICE.md THIRD_PARTY_LICENSES.md FONT_LICENSE.txt; do
    install -m 644 "$ROOT/dist/$f" "$stage/doc/$f"
done
install -m 644 "$src/doc/NOTICE" "$stage/doc/PHREEQC_NOTICE.txt"
install -m 755 "$ROOT/packaging/linux/install.sh" "$ROOT/packaging/linux/uninstall.sh" "$stage/"
install -m 644 "$ROOT/dist/README.md" "$stage/README.md"

archive="$OUT/$name.tar.gz"
tar -czf "$archive" -C "$WORK" "$name"

echo
echo "Built: $archive"
