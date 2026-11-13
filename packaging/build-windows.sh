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
HOOK=$ROOT/target/i686-pc-windows-gnu/dll/rea_close_hook.dll
WHEEL=$ROOT/target/i686-pc-windows-gnu/dll/rea_dinput.dll
[ -s "$EXE" ] || { echo "build the i686 release workspace first: $EXE" >&2; exit 1; }
# The SDK's close hook and wheel layer are dependencies, not workspace
# members: build their DLLs here (the release build of the workspace
# doesn't), and check their imports as CI checks the launcher's.
# Their `dll` feature holds DllMain and the exports; the launcher's own
# `layer-dlls` turns it on (Cargo takes no feature flags for packages
# outside the workspace), and --lib keeps the launcher's exe, which links
# the libraries without it, out of this build. The `dll` profile unwinds
# (Cargo.toml).
(cd "$ROOT" && cargo build --profile dll --locked --target i686-pc-windows-gnu --lib \
  -p cmr4-launcher -p rea-launcher-close-hook -p rea-launcher-dinput --features layer-dlls)
[ -s "$HOOK" ] || { echo "no close hook DLL: $HOOK" >&2; exit 1; }
[ -s "$WHEEL" ] || { echo "no wheel layer DLL: $WHEEL" >&2; exit 1; }
sh "$ROOT/sdk/packaging/check-imports.sh" "$EXE" "$HOOK" "$WHEEL"
rm -rf "$PKG"
mkdir -p "$PKG/linux"
cp "$EXE" "$PKG/CMR4Launcher.exe"
cp "$ROOT/README.md" "$ROOT/THIRD_PARTY.md" "$ROOT/LICENSE-MIT" "$ROOT/LICENSE-APACHE" "$PKG/"
cp -r "$ROOT/docs" "$PKG/docs"
cp "$ROOT/linux/cmr4-play" "$ROOT/linux/README.md" "$PKG/linux/"
# The shared play library cmr4-play sources, beside it.
cp "$ROOT/sdk/rea-play.sh" "$PKG/linux/rea-play.sh"
chmod +x "$PKG/linux/cmr4-play"
"$HERE/install-components.sh" "$PKG" "$CACHE"
cp "$HOOK" "$PKG/components/rea_close_hook.dll"
cp "$WHEEL" "$PKG/components/rea_dinput.dll"
sh "$ROOT/sdk/packaging/smoke-package.sh" "$PKG" "$HERE/package.allow"
rm -f "$OUT/$NAME.zip"
(cd "$OUT" && zip -qr "$NAME.zip" "$NAME")
unzip -tq "$OUT/$NAME.zip"
echo "built $OUT/$NAME.zip"
