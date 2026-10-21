#!/usr/bin/env bash
# Removes exactly what install.sh installed, plus files from earlier builds
# installed as "PhreeqcI (unofficial)". The per-user work and settings
# directories are left alone.
#
#   uninstall.sh                remove PQ Studio and any legacy install
#   uninstall.sh --legacy-only  remove only the legacy install (used by install.sh)
set -euo pipefail

PREFIX="${XDG_DATA_HOME:-$HOME/.local/share}"
ICON_ROOT="$PREFIX/icons/hicolor"

remove_install() {
    local app_id="$1" bin_name="$2" doc_name="$3"
    rm -f "$HOME/.local/bin/$bin_name"
    rm -rf "${HOME:?}/.local/lib/$bin_name"
    rm -f "$PREFIX/applications/$app_id.desktop"
    for size in 16 32 48 64 128 256 512; do
        rm -f "$ICON_ROOT/${size}x${size}/apps/$app_id.png"
    done
    rm -f "$ICON_ROOT/scalable/apps/$app_id.svg"
    rm -rf "${PREFIX:?}/doc/$doc_name"
}

remove_install "no.laerk.PhreeqciUnofficial" "phreeqci-linux" "phreeqci-linux"
if [[ "${1:-}" != "--legacy-only" ]]; then
    remove_install "no.laerk.PQStudio" "pq-studio" "pq-studio"
fi

if [[ -f "$ICON_ROOT/icon-theme.cache" ]] && command -v gtk-update-icon-cache >/dev/null; then
    gtk-update-icon-cache -f -t -q "$ICON_ROOT" || true
fi

if [[ "${1:-}" != "--legacy-only" ]]; then
    echo "Uninstalled PQ Studio. Settings (~/.config/pqstudio) and work files (~/.cache/pqstudio) were kept."
fi
