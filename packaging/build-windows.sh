#!/bin/sh
# Builds the Windows zip from the release executable and pinned components.
set -eu
[ "$#" -ge 2 ] && [ "$#" -le 3 ] || { echo "usage: $0 <version> <output-folder> [cache]" >&2; exit 2; }
HERE=$(dirname "$(readlink -f "$0")")
ROOT=$(cd "$HERE/.." && pwd)
VERSION=$1
case "$VERSION" in *[!A-Za-z0-9._-]*) echo "unsafe version: $VERSION" >&2; exit 2 ;; esac
mkdir -p "$2"
OUT=$(readlink -f "$2")
CACHE=${3:-${CMR4_CACHE:-$HOME/.cache/cmr4-launcher}}
NAME=CMR4Launcher-$VERSION
PKG=$OUT/$NAME
EXE=$ROOT/target/i686-pc-windows-gnu/release/CMR4Launcher.exe
HOOK=$ROOT/target/i686-pc-windows-gnu/release/cmr4close.dll
for built in "$EXE" "$HOOK"; do
  [ -s "$built" ] || { echo "build the i686 release workspace first: $built" >&2; exit 1; }
done
rm -rf "$PKG"
mkdir -p "$PKG/linux"
cp "$EXE" "$PKG/CMR4Launcher.exe"
cp "$ROOT/README.md" "$ROOT/THIRD_PARTY.md" "$ROOT/LICENSE-MIT" "$ROOT/LICENSE-APACHE" "$PKG/"
cp -r "$ROOT/docs" "$PKG/docs"
cp "$ROOT/linux/cmr4-play" "$ROOT/linux/README.md" "$PKG/linux/"
chmod +x "$PKG/linux/cmr4-play"
"$HERE/install-components.sh" "$PKG" "$CACHE"
cp "$HOOK" "$PKG/components/cmr4close.dll"
"$HERE/smoke-package.sh" "$PKG"
rm -f "$OUT/$NAME.zip"
(cd "$OUT" && zip -qr "$NAME.zip" "$NAME")
unzip -tq "$OUT/$NAME.zip"
echo "built $OUT/$NAME.zip"
