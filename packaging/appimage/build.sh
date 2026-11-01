#!/bin/sh
# Builds CMR4Launcher-x86_64.AppImage with portable Wine 11.20.
set -eu
[ "$#" = 2 ] || { echo "usage: $0 <unpacked-package-folder> <output-folder>" >&2; exit 2; }
PKG=$(readlink -f "$1")
mkdir -p "$2"
OUT=$(readlink -f "$2")
HERE=$(dirname "$(readlink -f "$0")")
CACHE=${CMR4_CACHE:-$HOME/.cache/cmr4-launcher}
WINE_VERSION=11.20
WINE_URL=https://github.com/Kron4ek/Wine-Builds/releases/download/$WINE_VERSION/wine-$WINE_VERSION-amd64-wow64.tar.xz
WINE_SHA=482a493c755dcea776199b86560e0fb0f969bb0275c67d88120b0b35b0df34b9
TOOL_URL=https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-x86_64.AppImage
TOOL_SHA=ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0
RUNTIME_URL=https://github.com/AppImage/type2-runtime/releases/download/continuous/runtime-x86_64
RUNTIME_SHA=617aa77314d57c8b8e4fae9c3f0b35af7b30d5529cb69e98f64f92510ed98a46

fetch() { # url sha256 filename
  mkdir -p "$CACHE"
  if [ ! -f "$CACHE/$3" ]; then
    curl -fL --retry 3 "$1" -o "$CACHE/$3.part"
    mv "$CACHE/$3.part" "$CACHE/$3"
  fi
  if ! echo "$2  $CACHE/$3" | sha256sum -c --quiet; then
    echo "checksum mismatch: $3" >&2; rm -f "$CACHE/$3"; exit 1
  fi
}
fetch "$WINE_URL" "$WINE_SHA" wine-11.20.tar.xz
fetch "$TOOL_URL" "$TOOL_SHA" appimagetool-1.9.1
fetch "$RUNTIME_URL" "$RUNTIME_SHA" runtime-x86_64
chmod +x "$CACHE/appimagetool-1.9.1"

APPDIR=$(mktemp -d)/CMR4Launcher.AppDir
trap 'rm -rf "$(dirname "$APPDIR")"' EXIT HUP INT TERM
mkdir -p "$APPDIR/usr/lib/cmr4" "$APPDIR/usr/lib/wine"
tar -xJf "$CACHE/wine-11.20.tar.xz" -C "$APPDIR/usr/lib/wine" --strip-components=1
rm -rf "$APPDIR/usr/lib/wine/include" "$APPDIR/usr/lib/wine/share/man"
for tool in function_grep.pl widl winebuild winecpp winedump wineg++ winegcc winemaker wmc wrc msidb; do
  rm -f "$APPDIR/usr/lib/wine/bin/$tool"
done
cp "$PKG/CMR4Launcher.exe" "$APPDIR/usr/lib/cmr4/"
cp -r "$PKG/components" "$APPDIR/usr/lib/cmr4/components"
cp "$PKG/linux/cmr4-play" "$APPDIR/usr/lib/cmr4/"
mkdir -p "$APPDIR/usr/share/doc/wine" "$APPDIR/usr/share/doc/cmr4"
cp "$HERE"/wine/* "$APPDIR/usr/share/doc/wine/"
cp "$PKG/README.md" "$PKG/THIRD_PARTY.md" "$PKG/LICENSE-MIT" "$PKG/LICENSE-APACHE" \
  "$APPDIR/usr/share/doc/cmr4/"
cp -r "$PKG/licenses" "$APPDIR/usr/share/doc/cmr4/licenses"
cp -r "$PKG/docs" "$APPDIR/usr/share/doc/cmr4/docs"
cp "$HERE/AppRun" "$APPDIR/AppRun"
cp "$HERE/cmr4-launcher.desktop" "$HERE/cmr4-launcher.png" "$APPDIR/"
chmod +x "$APPDIR/AppRun" "$APPDIR/usr/lib/cmr4/cmr4-play"
ARCH=x86_64 "$CACHE/appimagetool-1.9.1" --appimage-extract-and-run --no-appstream \
  --runtime-file "$CACHE/runtime-x86_64" "$APPDIR" "$OUT/CMR4Launcher-x86_64.AppImage"
[ -s "$OUT/CMR4Launcher-x86_64.AppImage" ]
echo "built $OUT/CMR4Launcher-x86_64.AppImage (Wine $WINE_VERSION)"
