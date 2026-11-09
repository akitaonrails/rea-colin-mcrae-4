#!/bin/sh
# Downloads only officially pinned compatibility archives and verifies them
# (rea-launcher-sdk pins.sh); prints each verified file.
set -eu
HERE=$(dirname "$(readlink -f "$0")")
CACHE=${1:-${CMR4_CACHE:-$HOME/.cache/cmr4-launcher}}
. "$HERE/../sdk/packaging/pins.sh"
PIN_FILE=$HERE/components.pins
PIN_CACHE=
for name in xidi vcredist dxvk dgvoodoo innoextract; do
  echo "verified $name: $(pin_fetch "$name" "$CACHE")"
done
