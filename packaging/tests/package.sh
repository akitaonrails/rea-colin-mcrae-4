#!/bin/sh
# Package allowlist smoke tests; no network, Wine, or game process.
set -eu
HERE=$(dirname "$(readlink -f "$0")")
T=$(mktemp -d)
trap 'rm -rf "$T"' EXIT HUP INT TERM
PKG=$T/pkg
mkdir -p "$PKG/components" "$PKG/linux" "$PKG/docs/images" "$PKG/licenses/dgVoodoo2" "$PKG/licenses/DXVK" "$PKG/licenses/Xidi/ThirdParty"
for file in CMR4Launcher.exe README.md docs/ENHANCEMENTS.md docs/RESEARCH.md docs/RUNTIME_NOTES.md docs/images/game.png docs/images/launcher.png docs/images/launcher-picture.png THIRD_PARTY.md LICENSE-MIT LICENSE-APACHE \
  components/D3D9.dll components/cmr4close.dll \
  components/dinput8.dll components/d3d11.dll components/dxgi.dll linux/cmr4-play linux/rea-play.sh linux/README.md \
  licenses/DXVK/LICENSE \
  licenses/dgVoodoo2/README.md \
  licenses/Xidi/LICENSE licenses/Xidi/README.md \
  licenses/Xidi/ThirdParty/Boost_LICENSE licenses/Xidi/ThirdParty/Hookshot_LICENSE \
  licenses/Xidi/ThirdParty/XstdBitSet_LICENSE; do
  echo fixture > "$PKG/$file"
done
chmod +x "$PKG/linux/cmr4-play"
"$HERE/../smoke-package.sh" "$PKG" >/dev/null
echo "ok   complete allowlisted package passes"
mkdir -p "$PKG/Game"
echo asset > "$PKG/Game/track.dat"
if "$HERE/../smoke-package.sh" "$PKG" >/dev/null 2>&1; then
  echo "unexpected game content passed" >&2; exit 1
fi
rm "$PKG/Game/track.dat"
echo "ok   unexpected game content fails"
echo not-a-game > "$PKG/cmr4.exe"
if "$HERE/../smoke-package.sh" "$PKG" >/dev/null 2>&1; then
  echo "game executable name passed" >&2; exit 1
fi
rm "$PKG/cmr4.exe"
echo "ok   game executable name fails"
rm "$PKG/components/dinput8.dll"
if "$HERE/../smoke-package.sh" "$PKG" >/dev/null 2>&1; then
  echo "missing component passed" >&2; exit 1
fi
echo "ok   missing required component fails"
# Every checksum-pinned download must name a fixed release, never a moving tag.
sh -n "$HERE/../appimage/build.sh"
if grep -n 'releases/download/continuous/' "$HERE/../appimage/build.sh" >&2; then
  echo "AppImage build pins a moving release tag" >&2; exit 1
fi
echo "ok   AppImage downloads pin fixed releases"
echo "all passed"
