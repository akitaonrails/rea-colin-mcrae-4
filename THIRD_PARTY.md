# Third-party notices

Compatibility binaries are fetched from the official pinned URLs in
`packaging/components.pins`; those archives are not committed. The sole
intentional exception is the documented `packaging/mirror/dgVoodoo2_80.zip`
containing only the tested dgVoodoo2 2.80 D3D9 compatibility DLL. Phase 3
packaging preserves these notices and the component license files in each package.

## Compiled into CMR4Launcher.exe

| Component | License | Source |
| --- | --- | --- |
| `sha2` and RustCrypto dependencies | MIT OR Apache-2.0 | https://github.com/RustCrypto/hashes |
| `windows-sys` | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `eframe`, `egui`, `winit`, `glutin`, `glow`, and dependencies | MIT OR Apache-2.0; some dependencies use Zlib/BSD (see `cargo tree`) | https://github.com/emilk/egui |
| `jpeg-decoder` | MIT OR Apache-2.0 | https://github.com/image-rs/jpeg-decoder |
| Rust standard library | MIT OR Apache-2.0 | https://github.com/rust-lang/rust |
| MinGW-w64 runtime portions linked by the GNU target | ZPL-2.1 / public domain | https://www.mingw-w64.org |

## Settings-window assets

| Asset | License and attribution | Source |
| --- | --- | --- |
| `assets/portrait.jpg` — Colin McRae, photo by Carolyn Williams (resized), credited in the window | CC BY 2.0 | https://commons.wikimedia.org/wiki/File:Colin_Mcrae_crop.jpg |
| `assets/fonts/Barlow-*.ttf`, `BarlowCondensed-*.ttf` | SIL OFL 1.1; notice in `assets/fonts/OFL.txt` | https://github.com/jpt/barlow |
| `packaging/appimage/cmr4-launcher.png` | Project icon (MIT OR Apache-2.0), set in Barlow | this repository |

## Bundled compatibility DLLs

| Component | Version and pin | License / notices | Official source |
| --- | --- | --- | --- |
| Xidi | 4.3.1; SHA-256 `78120f44baa816fc264fb722495776aab437a36be515e42345428b7f32ac17ff` | MIT; `licenses/Xidi/LICENSE` and `ThirdParty` notices | https://github.com/samuelgr/Xidi/releases/tag/v4.3.1 |
| dgVoodoo2 | Tested 2.80 D3D9.dll; SHA-256 `6f0eae6d0fbe836490b1aa3361304d2398fdb5d815a929e126b5ae7b13b16e47` | Copyright Dege; provenance and notice in `packaging/mirror/README.md` and `licenses/dgVoodoo2/README.md` | https://dege.freeweb.hu/dgVoodoo2/dgVoodoo2/ |
| DXVK | 3.1.1; SHA-256 `40565b4a724aadc4433fa4e010b4b23916d9b1f1baeee64e17186db94f54e608` | zlib/libpng; `licenses/DXVK/LICENSE`; x32 d3d11/dxgi installed only under Wine/Proton | https://github.com/doitsujin/dxvk/releases/tag/v3.1.1 |

Xidi 5.0 is intentionally not used. dgVoodoo2 2.87.6 is not used: controlled
CMR04 tests crashed with it. The package carries only the tested 2.80 D3D9
mirror. Final Linux/AppImage 4K/menu and virtual-pad checks passed; native
Windows, analog/full-stage driving, separate Proton/Deck, raw-desktop visuals,
and protected retail media remain unknown.

## Linux AppImage

| Component | License | Source |
| --- | --- | --- |
| Wine 11.20 (amd64, WoW64), Kron4ek build of unmodified WineHQ sources; source and license notices under `usr/share/doc/wine` | LGPL-2.1-or-later | https://dl.winehq.org/wine/source/11.x/wine-11.20.tar.xz and https://github.com/Kron4ek/Wine-Builds |
| AppImage runtime added by appimagetool 1.9.1 | MIT | https://github.com/AppImage/type2-runtime |

## Not supplied

No Codemasters executable, DLL, save, media, track, car data, or other game
asset is included. DirectX/system libraries not explicitly listed above come
from Windows, Wine, Proton, or the user's own installation.
