# CMR04 research and component rationale

## Established recipe and executable boundary

The launcher uses the tested CMR04 setup rather than generic CMR1 options:

- `cmr4.exe` is PE32. The 16:9 Low patch replaces exactly one occurrence of
  `04 00 00 80 3f e8` with `04 00 00 40 3f e8`, only in the private runtime.
- Width and Height are written every launch. Widescreen requires exact 16:9;
  Original, 4:3, 16:10, and ultrawide use widescreen off rather than distortion.
- The default is the largest exact 16:9 mode fitting the active screen in
  landscape, capped at 3840x2160. Unknown screens use 1920x1080.
- CMR04 requires install path strings and first-run defaults in its 32-bit
  registry key. They are seeded without exposing language/graphics options.
- Raw Wine uses borderless dgVoodoo fullscreen; native Windows and gamescope
  use normal fullscreen. Scaling preserves aspect with letter/pillarboxing.
- Xidi maps left stick/D-pad to arrows, RT/LT to pedals, A/Start to Enter,
  B/Back to Esc, X to Space, and Y to C.

References:

- [PCGamingWiki: Colin McRae Rally 04](https://www.pcgamingwiki.com/wiki/Colin_McRae_Rally_04)
- [Xidi](https://github.com/samuelgr/Xidi)
- [dgVoodoo2 official project](https://dege.freeweb.hu/dgVoodoo2/dgVoodoo2/)
- [Dege's official dgVoodoo2 repository](https://github.com/dege-diosg/dgVoodoo2)
- [DXVK](https://github.com/doitsujin/dxvk)

The in-game Advanced Graphics menu rewrote the tested resolution and hung;
users are advised to avoid it. A 1920x1440 fallback also failed in the recipe
and is not presented as known-good.

## Controlled runtime evidence

The exact sequence that determined the final policy:

1. dgVoodoo2 2.87.6 crashed even after required registry paths were present
   and D3D11 was forced. It is not used anywhere in the final package.
2. Fresh-prefix testing showed that CMR04 also needed `AUDIO_PATH`, `CD_PATH`,
   `INSTALL_PATH`, and `VIDEO_PATH` pointed at the private runtime. The
   launcher now always writes those paths and seeds game defaults only if
   missing, preserving player choices thereafter.
3. dgVoodoo2 2.80 plus DXVK reached CMR04's title and 3D attract demo at 4K
   after those paths were seeded. The measured swapchain was 3840x2160.
4. The exact final package copied 1661 files on its first launch and zero on
   its second. It reached the 4K title/main menu on an RTX 5090 without a
   watermark, and source executable/settings hashes remained unchanged.
5. Settings-window capture was correct after final-size creation under Wine.
6. The exact AppImage reached 4K title/options, gearbox, and car-class screens
   with a virtual Xbox pad using A and the D-pad, verifying Xidi menu input.
   Analog steering/pedals and a full stage were not exercised.
7. Raw desktop Wine did not crash and had a valid swapchain, but its capture
   was interrupted and is not visual verification. Native Windows, a full
   stage, separate Proton/Deck, and protected retail media remain unknown.

A screenshot from another session was not attributable to the failed run and
was excluded from evidence.

## Compatibility pins and narrow mirror

`packaging/components.pins` records official downloads. Archives are cached,
SHA-256 checked, and exact members are installed. No compatibility binary is
taken from the installed Codemasters game.

### dgVoodoo2 2.80

The official v2.8 release has only the unrelated dgVoodoo API archive; the
full 2.80 binary distribution is unavailable from Dege. The documented narrow
exception is `packaging/mirror/dgVoodoo2_80.zip`, containing only the tested
third-party `MS/x86/D3D9.dll` and no game data:

- DLL SHA-256: `6f0eae6d0fbe836490b1aa3361304d2398fdb5d815a929e126b5ae7b13b16e47`
- ZIP SHA-256: `cec7e2c8eeaec0065cc8f41d8e0f176ae28d55074c4295ce3670589dc9f29d30`
- Provenance and official links: `packaging/mirror/README.md`

CMR04 imports D3D9 and the working test needs only this dgVoodoo DLL. Unused
D3D8.dll, D3DImm.dll, and DDraw.dll are not packaged or installed. The
configuration disables the dgVoodoo watermark.

### Xidi 4.3.1

- URL: <https://github.com/samuelgr/Xidi/releases/download/v4.3.1/Xidi-v4.3.1.zip>
- SHA-256: `78120f44baa816fc264fb722495776aab437a36be515e42345428b7f32ac17ff`
- Payload: `Xidi-v4.3.1/Win32/dinput8.dll` plus license notices.

Xidi 5.0 is deliberately not adopted without a required feature or CMR04
runtime evidence.

### DXVK 3.1.1

- URL: <https://github.com/doitsujin/dxvk/releases/download/v3.1.1/dxvk-3.1.1.tar.gz>
- SHA-256: `40565b4a724aadc4433fa4e010b4b23916d9b1f1baeee64e17186db94f54e608`
- Payloads: `dxvk-3.1.1/x32/d3d11.dll` and `dxgi.dll`, only under Wine/Proton.
- License: zlib/libpng from the official v3.1.1 repository.

Native Windows does not require or install DXVK and retains dgVoodoo to system
D3D11. Ownership/source-byte conflict checks apply to every wrapper.

## Display and Linux details

CMR3's measured Wine results showed exclusive fullscreen selecting a portrait
monitor and cropping output, while borderless or gamescope worked. CMR04 uses
`FullscreenAttributes=fake` only for raw Wine. `GAMESCOPE_WAYLAND_DISPLAY`
selects context without adding a user option.

Wine's settings window seeds `UseEGL=N` only if absent and is created at its
final scaled size. Linux passes both SDL controller and joystick background
event variables independently, preserving existing user values.

See [RUNTIME_NOTES.md](RUNTIME_NOTES.md) for remaining verification.
