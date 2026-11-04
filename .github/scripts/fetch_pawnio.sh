#!/usr/bin/env bash
# Downloads the pinned PawnIO installer and modules (packaging/pins.txt),
# refuses anything whose SHA-256 differs, and lays them out as packages ship
# them: OUT/PawnIO_setup.exe and OUT/pawnio-modules/{*.bin,COPYING}.
# usage: fetch_pawnio.sh OUT
set -euo pipefail
out=$1
. "$(dirname "$0")/../../packaging/pins.txt"
mkdir -p "$out/pawnio-modules"
tmp=$(mktemp -d)
curl -fsSL --retry 3 "$PAWNIO_SETUP_URL" -o "$out/PawnIO_setup.exe"
curl -fsSL --retry 3 "$PAWNIO_MODULES_URL" -o "$tmp/modules.zip"
echo "$PAWNIO_SETUP_SHA256  $out/PawnIO_setup.exe" | sha256sum -c -
echo "$PAWNIO_MODULES_SHA256  $tmp/modules.zip" | sha256sum -c -
python -m zipfile -e "$tmp/modules.zip" "$out/pawnio-modules"
test -f "$out/pawnio-modules/COPYING" || { echo "modules zip has no COPYING"; exit 1; }
printf 'PawnIO modules %s, official signed build.\nSource: https://github.com/namazso/PawnIO.Modules/tree/%s\nLicense: LGPL-2.1-or-later (COPYING)\n' \
  "$PAWNIO_MODULES_VERSION" "$PAWNIO_MODULES_VERSION" > "$out/pawnio-modules/SOURCE.txt"
rm -rf "$tmp"
ls -l "$out" "$out/pawnio-modules"
