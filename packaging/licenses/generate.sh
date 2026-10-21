#!/usr/bin/env bash
# Regenerates dist/THIRD_PARTY_LICENSES.md. Needs: cargo install cargo-about --features cli
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
cargo about generate -c packaging/licenses/about.toml \
    packaging/licenses/third-party.md.hbs -o dist/THIRD_PARTY_LICENSES.md
echo "wrote dist/THIRD_PARTY_LICENSES.md"
