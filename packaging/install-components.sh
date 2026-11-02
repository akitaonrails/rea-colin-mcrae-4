#!/bin/sh
# Installs exact archive members into an unpacked package; never uses game files.
set -eu
[ "$#" -ge 1 ] && [ "$#" -le 2 ] || { echo "usage: $0 <package-folder> [cache]" >&2; exit 2; }
HERE=$(dirname "$(readlink -f "$0")")
PKG=$(readlink -f "$1")
CACHE=${2:-${CMR4_CACHE:-$HOME/.cache/cmr4-launcher}}
"$HERE/fetch-components.sh" "$CACHE"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT HUP INT TERM
mkdir -p "$TMP/dgvoodoo" "$TMP/xidi" "$TMP/dxvk" "$PKG/components" \
  "$PKG/licenses/dgVoodoo2" "$PKG/licenses/Xidi/ThirdParty" "$PKG/licenses/DXVK"

# The tested 2.80 archive (our thirdparty release): one member, and a
# required DLL digest.
unzip -q "$CACHE/dgVoodoo2_80.zip" 'MS/x86/D3D9.dll' -d "$TMP/dgvoodoo"
unzip -q "$CACHE/Xidi-v4.3.1.zip" \
  'Xidi-v4.3.1/Win32/dinput8.dll' 'Xidi-v4.3.1/LICENSE' 'Xidi-v4.3.1/README.md' \
  'Xidi-v4.3.1/ThirdParty/*' -d "$TMP/xidi"
tar -xzf "$CACHE/dxvk-3.1.1.tar.gz" -C "$TMP/dxvk" \
  dxvk-3.1.1/x32/d3d11.dll dxvk-3.1.1/x32/dxgi.dll

[ -s "$TMP/dgvoodoo/MS/x86/D3D9.dll" ]
cp "$TMP/dgvoodoo/MS/x86/D3D9.dll" "$PKG/components/D3D9.dll"
. "$HERE/../sdk/packaging/pins.sh"
PIN_FILE=$HERE/components.pins
pin_verify "$PKG/components/D3D9.dll" "$(pin_sha dgvoodoo-d3d9)"
[ -s "$TMP/xidi/Xidi-v4.3.1/Win32/dinput8.dll" ]
cp "$TMP/xidi/Xidi-v4.3.1/Win32/dinput8.dll" "$PKG/components/dinput8.dll"
for name in d3d11.dll dxgi.dll; do
  [ -s "$TMP/dxvk/dxvk-3.1.1/x32/$name" ]
  cp "$TMP/dxvk/dxvk-3.1.1/x32/$name" "$PKG/components/$name"
done
cp "$HERE/mirror/README.md" "$PKG/licenses/dgVoodoo2/README.md"
cp "$TMP/xidi/Xidi-v4.3.1/LICENSE" "$TMP/xidi/Xidi-v4.3.1/README.md" "$PKG/licenses/Xidi/"
cp "$TMP/xidi/Xidi-v4.3.1/ThirdParty/"* "$PKG/licenses/Xidi/ThirdParty/"
cp "$HERE/DXVK-LICENSE" "$PKG/licenses/DXVK/LICENSE"
echo "installed pinned components into $PKG/components"
