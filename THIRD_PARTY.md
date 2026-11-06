# Third-party notices

Compatibility binaries are fetched at package time from the pinned URLs in
`packaging/components.pins` and checked by SHA-256; no archive is committed.
The one archive we host is `dgVoodoo2_80.zip` (only the tested dgVoodoo2
2.80 `D3D9.dll`) in this repository's `thirdparty` release: dgVoodoo2 has
no licence text and its 2.80 archive is gone upstream, so hosting it is the
owner's recorded exception (see `packaging/mirror/README.md`). Packages
carry these notices and the components' licence files.

## Compiled into CMR4Launcher.exe

| Component | License | Source |
| --- | --- | --- |
| `rea-launcher`, `rea-launcher-win32`, `rea-launcher-ui` (rea-launcher-sdk v0.2.0) | MIT OR Apache-2.0 | https://github.com/akitaonrails/rea-launcher-sdk |
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
| Barlow fonts (`Barlow-*.ttf`, `BarlowCondensed-*.ttf`), embedded from the shared `rea-launcher-ui` crate (`crates/rea-launcher-ui/assets/fonts` in rea-launcher-sdk) | SIL OFL 1.1; notice in that folder's `OFL.txt` | https://github.com/jpt/barlow |
| `packaging/appimage/cmr4-launcher.png` | Project icon (MIT OR Apache-2.0), set in Barlow | this repository |

The portrait's source was checked on 2026-11-02: Wikimedia Commons
`File:Colin_Mcrae_crop.jpg`, "Cropped from" Flickr photo 1429137255 by
Carolyn Williams (account `carolyn.will`), CC BY 2.0
(https://www.flickr.com/photos/8365241@N02/1429137255/). Commons notes
personality rights for the people shown.

## README screenshots and video

| File | What | Author and licence |
| --- | --- | --- |
| `docs/images/launcher-play.jpg`, `docs/images/launcher-picture.jpg`, `docs/images/launcher-controls.jpg` | The settings window (our own UI; the photo in it as above) | this project, MIT OR Apache-2.0 |
| `docs/images/gameplay.jpg` | A screenshot of Colin McRae Rally 04 running through the launcher | game imagery © Codemasters; captured by the project owner to show the launcher at work |
| Showcase video and poster (hosted at `new-uploads-akitaonrails.s3.us-east-2.amazonaws.com`, linked from the README, not in the repository) | Gameplay recorded by the project owner | game imagery © Codemasters |

## Community fixes

| Fix | Author | How it is used |
| --- | --- | --- |
| CMR04 16:9 "Low" widescreen fix (one camera constant in `cmr4.exe`) | AuToMaNiAk005 (found via PCGamingWiki) | the same byte change, written as our patch table `src/edition.rs`; applied to the private copy of identified builds only; no file of the fix is shipped |

## Bundled compatibility DLLs

| Component | Version and pin | License / notices | Official source |
| --- | --- | --- | --- |
| Xidi | 4.3.1; SHA-256 `78120f44baa816fc264fb722495776aab437a36be515e42345428b7f32ac17ff` | BSD-3-Clause; `licenses/Xidi/LICENSE` and `ThirdParty` notices | https://github.com/samuelgr/Xidi/releases/tag/v4.3.1 |
| Microsoft Visual C++ 2015–2022 Redistributable (x86) | `msvcp140.dll`, `msvcp140_atomic_wait.dll`, `concrt140.dll`, `vcruntime140.dll` from `VC_redist.x86.exe` (SHA-256 `0c09f2611660441084ce0df425c51c11e147e6447963c3690f97e0b25c55ed64`) | Microsoft Visual Studio Distributable Code terms (redistributable binaries) | https://learn.microsoft.com/en-us/cpp/windows/latest-supported-vc-redist |
| dgVoodoo2 | Tested 2.80 D3D9.dll; SHA-256 `6f0eae6d0fbe836490b1aa3361304d2398fdb5d815a929e126b5ae7b13b16e47`, from `dgVoodoo2_80.zip` (`cec7e2c8…9d30`) in this repository's `thirdparty` release | Copyright Dege; **no licence text**: bundled by the owner's recorded exception (the 2.80 archive is gone upstream and 2.87.6 crashed CMR04); provenance and notice in `packaging/mirror/README.md` and `licenses/dgVoodoo2/README.md` | https://dege.freeweb.hu/dgVoodoo2/dgVoodoo2/ |
| DXVK | 3.1.1; SHA-256 `40565b4a724aadc4433fa4e010b4b23916d9b1f1baeee64e17186db94f54e608` | zlib/libpng; `licenses/DXVK/LICENSE`; x32 d3d11/dxgi installed only under Wine/Proton | https://github.com/doitsujin/dxvk/releases/tag/v3.1.1 |

Xidi 5.0 is intentionally not used. dgVoodoo2 2.87.6 is not used: controlled
CMR04 tests crashed with it. What has and hasn't been verified in the game
is in `docs/RUNTIME_NOTES.md`.

## Linux AppImage

| Component | License | Source |
| --- | --- | --- |
| Wine 11.20 (amd64, WoW64), Kron4ek build of unmodified WineHQ sources; source and license notices under `usr/share/doc/wine` | LGPL-2.1-or-later | https://dl.winehq.org/wine/source/11.x/wine-11.20.tar.xz and https://github.com/Kron4ek/Wine-Builds |
| AppImage type2 runtime, release `20251108`, SHA-256 `2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d`, added by appimagetool 1.9.1 | MIT | https://github.com/AppImage/type2-runtime/releases/tag/20251108 |
| appimagetool 1.9.1 (build tool only; not in the package), pinned by the SDK's `appimage-wine.sh` | MIT | https://github.com/AppImage/appimagetool |
| `rea-play.sh` (the Linux play library, next to `cmr4-play`) | MIT OR Apache-2.0 | rea-launcher-sdk |

## Not supplied

No Codemasters executable, DLL, save, media, track, car data, or other game
asset is included. DirectX/system libraries not explicitly listed above come
from Windows, Wine, Proton, or the user's own installation.
