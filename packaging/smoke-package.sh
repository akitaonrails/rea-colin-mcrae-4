#!/bin/sh
# Requires every package file and rejects every unexpected file (including game data).
set -eu
[ "$#" = 1 ] || { echo "usage: $0 <package-folder>" >&2; exit 2; }
PKG=$(readlink -f "$1")
[ -d "$PKG" ]
required='CMR4Launcher.exe
README.md
docs/ENHANCEMENTS.md
docs/RESEARCH.md
docs/RUNTIME_NOTES.md
docs/images/game.png
docs/images/launcher-picture.png
docs/images/launcher.png
THIRD_PARTY.md
LICENSE-MIT
LICENSE-APACHE
components/D3D9.dll
components/cmr4close.dll
components/dinput8.dll
components/d3d11.dll
components/dxgi.dll
linux/cmr4-play
linux/rea-play.sh
linux/README.md
licenses/dgVoodoo2/README.md
licenses/DXVK/LICENSE
licenses/Xidi/LICENSE
licenses/Xidi/README.md
licenses/Xidi/ThirdParty/Boost_LICENSE
licenses/Xidi/ThirdParty/Hookshot_LICENSE
licenses/Xidi/ThirdParty/XstdBitSet_LICENSE'
echo "$required" | while IFS= read -r file; do
  [ -s "$PKG/$file" ] || { echo "missing or empty package file: $file" >&2; exit 1; }
done
expected=$(echo "$required" | LC_ALL=C sort)
actual=$(cd "$PKG" && find . -type f -print | sed 's|^\./||' | LC_ALL=C sort)
if [ "$actual" != "$expected" ]; then
  echo "unexpected package inventory (game assets and unlisted files are forbidden):" >&2
  echo "$actual" >&2
  exit 1
fi
[ -x "$PKG/linux/cmr4-play" ]
echo "package inventory and required files verified: $PKG"
