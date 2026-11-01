# Project rules

This repository builds a launcher for the user's own PC copy of Colin McRae
Rally 04 on modern Windows and under Proton/Wine. It is not a distribution of
the game.

## Non-negotiable boundaries

- Never commit, vendor, embed, or publish original executables, DLLs, media,
  archives, tracks, car data, or other game assets, or anything copied from them.
- The installed game folder is read-only. All patches, generated configuration,
  saves, and writable game files belong in the separate persistent runtime.
- Never commit a developer's absolute game path. Code and examples take an
  explicit path instead.
- Put local experiments, traces, and investigation output under `.work/`.
- Do not run the real game as part of ordinary host tests.

## Code and verification

- Use Rust for launcher code. Keep `unsafe` and Win32 FFI in narrow modules;
  portable configuration, patching, and runtime planning stay host-testable.
- The only widescreen patch is the tested 16:9 Low signature. Missing or
  ambiguous signatures must fail safely and report SHA-256, never guess a site.
- Preserve game-owned runtime data across source syncs. Do not overwrite user
  DLLs or other custom runtime files merely to install launcher components.
- Keep third-party component payloads and downloaded archives out of this
  repository. Packaging fetches exact members from official pinned URLs with
  SHA-256 verification into a cache. Record every pin, license, and source in
  `packaging/components.pins` and `THIRD_PARTY.md`.
- Sole mirror exception: `packaging/mirror/dgVoodoo2_80.zip` may contain only
  the tested dgVoodoo2 2.80 `MS/x86/D3D9.dll` documented in
  `packaging/mirror/README.md`. No other mirror or compatibility payload is
  allowed; the official 2.80 full archive is unavailable and 2.87.6 failed.
- Build packages only in CI output directories or `.work/`, never the repo root.
  Package smoke checks must reject unlisted files and every game asset.
- Host tests and a Windows cross-build establish only portable logic and
  compilation. Do not claim registry, wrapper, controller, graphics, or gameplay
  behavior is verified until it has been exercised in the real target runtime.
- Record exact verification commands and limitations with each milestone.
