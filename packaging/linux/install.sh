#!/usr/bin/env bash
# Per-user install of PQ Studio, a graphical interface for the USGS PHREEQC
# engine. No root needed. Works in two places:
#
#   In an unpacked release tarball (PQ-Studio-<version>-linux-<arch>):
#     ./install.sh           installs the app together with its bundled
#                            PHREEQC engine and databases
#
#   In the source repository (uses the phreeqc you have installed):
#     packaging/linux/install.sh            build (cargo --release) and install
#     packaging/linux/install.sh --no-build install an already-built binary
#
# Installs only files named after APP_ID / BIN_NAME below; never touches any
# other application's files. uninstall.sh reverses it.
set -euo pipefail

APP_ID="no.laerk.PQStudio"
BIN_NAME="pq-studio"

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PREFIX="${XDG_DATA_HOME:-$HOME/.local/share}"
BIN_DIR="$HOME/.local/bin"
LIB_DIR="$HOME/.local/lib/$BIN_NAME"
APPS_DIR="$PREFIX/applications"
ICON_ROOT="$PREFIX/icons/hicolor"
DOC_DIR="$PREFIX/doc/$BIN_NAME"

if [[ -x "$HERE/pq-studio" && -x "$HERE/phreeqc" ]]; then
    BUNDLED=1
    BUILT="$HERE/pq-studio"
    ICONS="$HERE/icons"
    DOCS="$HERE/doc"
else
    BUNDLED=0
    REPO_ROOT="$(cd "$HERE/../.." && pwd)"
    if [[ "${1:-}" != "--no-build" ]]; then
        (cd "$REPO_ROOT" && cargo build --release)
    fi
    BUILT="$REPO_ROOT/target/release/pq-studio"
    ICONS="$REPO_ROOT/assets/icon"
    DOCS="$REPO_ROOT/dist"
fi
[[ -x "$BUILT" ]] || { echo "error: $BUILT not found; run without --no-build" >&2; exit 1; }

# Earlier builds were installed as "PhreeqcI (unofficial)"; remove them first.
"$HERE/uninstall.sh" --legacy-only

# Remove first rather than overwrite: the old file may be a symlink, or the
# running program.
rm -f "$BIN_DIR/$BIN_NAME"
rm -rf "$LIB_DIR"
mkdir -p "$BIN_DIR"
if [[ "$BUNDLED" == 1 ]]; then
    # App, engine and databases stay together; the app finds the engine and
    # databases next to its own (resolved) path.
    install -Dm755 "$BUILT" "$LIB_DIR/$BIN_NAME"
    install -Dm755 "$HERE/phreeqc" "$LIB_DIR/phreeqc"
    mkdir -p "$LIB_DIR/database"
    install -m644 "$HERE"/database/*.dat "$LIB_DIR/database/"
    ln -s "$LIB_DIR/$BIN_NAME" "$BIN_DIR/$BIN_NAME"
else
    install -Dm755 "$BUILT" "$BIN_DIR/$BIN_NAME"
fi

for size in 16 32 48 64 128 256 512; do
    install -Dm644 "$ICONS/pq-studio-$size.png" "$ICON_ROOT/${size}x${size}/apps/$APP_ID.png"
done
install -Dm644 "$ICONS/pq-studio.svg" "$ICON_ROOT/scalable/apps/$APP_ID.svg"

for f in README.md LICENSE NOTICE.md THIRD_PARTY_LICENSES.md FONT_LICENSE.txt; do
    install -Dm644 "$DOCS/$f" "$DOC_DIR/$f"
done
if [[ -f "$DOCS/PHREEQC_NOTICE.txt" ]]; then
    install -Dm644 "$DOCS/PHREEQC_NOTICE.txt" "$DOC_DIR/PHREEQC_NOTICE.txt"
fi

# Desktop entry Exec is an absolute, quoted path: launchers don't reliably
# put ~/.local/bin on PATH.
exec_path="$BIN_DIR/$BIN_NAME"
exec_path="${exec_path//\\/\\\\}"
exec_path="${exec_path//\"/\\\"}"
mkdir -p "$APPS_DIR"
cat > "$APPS_DIR/$APP_ID.desktop" <<EOF
[Desktop Entry]
Type=Application
Version=1.5
Name=PQ Studio
GenericName=Geochemical Modeling
Comment=Graphical interface for the USGS PHREEQC geochemical engine
Exec="$exec_path"
Icon=$APP_ID
Terminal=false
Categories=Science;Chemistry;
Keywords=PHREEQC;geochemistry;speciation;aqueous;hydrogeology;
StartupNotify=true
StartupWMClass=$APP_ID
EOF
chmod 644 "$APPS_DIR/$APP_ID.desktop"

# Only refresh an icon cache that already exists; creating a new one in a
# shared hicolor tree can leave other apps' icons stale later.
if [[ -f "$ICON_ROOT/icon-theme.cache" ]] && command -v gtk-update-icon-cache >/dev/null; then
    gtk-update-icon-cache -f -t -q "$ICON_ROOT" || true
fi

if command -v desktop-file-validate >/dev/null; then
    desktop-file-validate "$APPS_DIR/$APP_ID.desktop"
fi

echo "Installed:"
echo "  $BIN_DIR/$BIN_NAME"
if [[ "$BUNDLED" == 1 ]]; then
    echo "  $LIB_DIR/ (app, PHREEQC engine, databases)"
fi
echo "  $APPS_DIR/$APP_ID.desktop"
echo "  $ICON_ROOT/{16..512}/apps/$APP_ID.png, scalable/apps/$APP_ID.svg"
echo "  $DOC_DIR/"
if [[ "$BUNDLED" == 0 ]] && ! command -v phreeqc >/dev/null && [[ ! -x /usr/bin/phreeqc ]]; then
    echo "note: the phreeqc engine was not found; install it (see README) or set PHREEQC_BIN." >&2
fi
