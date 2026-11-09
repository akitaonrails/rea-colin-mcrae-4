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
unzip -q "$CACHE/Xidi-v5.0.0.zip" \
  'Xidi-v5.0.0/Win32/dinput8.dll' 'Xidi-v5.0.0/Win32/Xidi.32.dll' \
  'Xidi-v5.0.0/LICENSE' 'Xidi-v5.0.0/README.md' \
  'Xidi-v5.0.0/ThirdParty/*' -d "$TMP/xidi"
tar -xzf "$CACHE/dxvk-3.1.1.tar.gz" -C "$TMP/dxvk" \
  dxvk-3.1.1/x32/d3d11.dll dxvk-3.1.1/x32/dxgi.dll

[ -s "$TMP/dgvoodoo/MS/x86/D3D9.dll" ]
cp "$TMP/dgvoodoo/MS/x86/D3D9.dll" "$PKG/components/D3D9.dll"
. "$HERE/../sdk/packaging/pins.sh"
PIN_FILE=$HERE/components.pins
pin_verify "$PKG/components/D3D9.dll" "$(pin_sha dgvoodoo-d3d9)"
# Xidi 5: the dinput8.dll wrapper and the main binary it loads.
for name in dinput8.dll Xidi.32.dll; do
  [ -s "$TMP/xidi/Xidi-v5.0.0/Win32/$name" ]
  cp "$TMP/xidi/Xidi-v5.0.0/Win32/$name" "$PKG/components/$name"
done
cabextract -q -d "$TMP/vc" "$CACHE/VC_redist.x86.exe"
for cab in "$TMP"/vc/a*; do
  if cabextract -l "$cab" 2>/dev/null | grep -q 'vcruntime140.dll_x86'; then
    cabextract -q -d "$TMP/vcdll" -F 'msvcp140.dll_x86' -F 'msvcp140_atomic_wait.dll_x86' \
      -F 'concrt140.dll_x86' -F 'vcruntime140.dll_x86' "$cab"
    break
  fi
done
for dll in msvcp140 msvcp140_atomic_wait concrt140 vcruntime140; do
  [ -s "$TMP/vcdll/$dll.dll_x86" ]
  cp "$TMP/vcdll/$dll.dll_x86" "$PKG/components/$dll.dll"
done
for name in d3d11.dll dxgi.dll; do
  [ -s "$TMP/dxvk/dxvk-3.1.1/x32/$name" ]
  cp "$TMP/dxvk/dxvk-3.1.1/x32/$name" "$PKG/components/$name"
done
cp "$HERE/mirror/README.md" "$PKG/licenses/dgVoodoo2/README.md"
cp "$TMP/xidi/Xidi-v5.0.0/LICENSE" "$TMP/xidi/Xidi-v5.0.0/README.md" "$PKG/licenses/Xidi/"
cp "$TMP/xidi/Xidi-v5.0.0/ThirdParty/"* "$PKG/licenses/Xidi/ThirdParty/"
cp "$HERE/DXVK-LICENSE" "$PKG/licenses/DXVK/LICENSE"
# innoextract 1.9: components/innoextract.exe (checked against its pin) and
# licenses/innoextract/*.txt; it unpacks the supported installer.
. "$HERE/../sdk/packaging/innoextract.sh"
innoextract_bundle "$PKG" "$CACHE"
echo "installed pinned components into $PKG/components"
