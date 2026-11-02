#!/bin/sh
# Package allowlist and pin smoke tests; no network, Wine, or game process.
set -eu
HERE=$(dirname "$(readlink -f "$0")")
ROOT=$HERE/../..
SMOKE="sh $ROOT/sdk/packaging/smoke-package.sh"
ALLOW=$HERE/../package.allow
T=$(mktemp -d)
trap 'rm -rf "$T"' EXIT HUP INT TERM
PKG=$T/pkg
# A package with every allowlisted file.
awk '!/^[ \t]*(#|$)/ { print $1 }' "$ALLOW" | while IFS= read -r file; do
  mkdir -p "$PKG/$(dirname "$file")"
  echo fixture > "$PKG/$file"
done
chmod +x "$PKG/linux/cmr4-play"
$SMOKE "$PKG" "$ALLOW" >/dev/null
echo "ok   complete allowlisted package passes"
for required in linux/rea-play.sh components/cmr4close.dll licenses/Xidi/LICENSE; do
  grep -qx "$required\( .*\)\{0,1\}" "$ALLOW" || { echo "$required is not allowlisted" >&2; exit 1; }
done
echo "ok   the play library, close hook and licences are shipped"
mkdir -p "$PKG/Game"
echo asset > "$PKG/Game/track.dat"
if $SMOKE "$PKG" "$ALLOW" >/dev/null 2>&1; then
  echo "unexpected game content passed" >&2; exit 1
fi
rm -r "$PKG/Game"
echo "ok   unexpected game content fails"
echo not-a-game > "$PKG/cmr4.exe"
if $SMOKE "$PKG" "$ALLOW" >/dev/null 2>&1; then
  echo "game executable name passed" >&2; exit 1
fi
rm "$PKG/cmr4.exe"
echo "ok   game executable name fails"
chmod -x "$PKG/linux/cmr4-play"
if $SMOKE "$PKG" "$ALLOW" >/dev/null 2>&1; then
  echo "non-executable helper passed" >&2; exit 1
fi
chmod +x "$PKG/linux/cmr4-play"
echo "ok   the helper must be executable"
rm "$PKG/components/dinput8.dll"
if $SMOKE "$PKG" "$ALLOW" >/dev/null 2>&1; then
  echo "missing component passed" >&2; exit 1
fi
echo "ok   missing required component fails"
# Every pin parses; checksum-pinned downloads name fixed releases, never a
# moving tag.
. "$ROOT/sdk/packaging/pins.sh"
PIN_FILE=$HERE/../components.pins
for name in xidi dxvk dgvoodoo-d3d9; do pin_sha "$name" >/dev/null; done
echo "ok   component pins parse"
for pins in "$HERE/../components.pins" "$ROOT/sdk/packaging/appimage/tools.pins"; do
  if grep -n 'releases/download/continuous/' "$pins" >&2; then
    echo "$pins pins a moving release tag" >&2; exit 1
  fi
done
echo "ok   downloads pin fixed releases"
echo "all passed"
