# Colin McRae Rally 04 launcher — instructions for coding agents

This repository makes the player's own PC copy of **Colin McRae Rally 04**
run on modern Windows and under Proton/Wine with a settings window,
sensible defaults and no manual steps. It is not a distribution of the
game. It is built on
[rea-launcher-sdk](https://github.com/akitaonrails/rea-launcher-sdk) at the
tag in `sdk/VERSION`.

**Read `sdk/docs/STANDARDS.md` before any work; the playbooks are in
`sdk/docs/playbooks/`** (vendored from the SDK at the tag in `sdk/VERSION`).

## Always

- The game's files are read-only: never committed, copied into the repo or published.
- Everything outside this repository is the user's and read-only (game installs,
  other repos, Wine prefixes, saves, home configs, containers, the SDK checkout)
  unless the user asked for or approved that specific change: back up first,
  never delete by a broad glob, ask when unsure.
- Headless by default; open a visible window only when a check needs a person, and say so first.
- Never print, log or commit credentials.
- Push, tag, release or post only with the user's go-ahead.
- Research community fixes first (`docs/RESEARCH.md`); patch only what nobody solved.
- Never edit `sdk/`; SDK gaps and bugs go to the SDK repository as issues or PRs.
- Clean up after a release or a long session: `sdk/scripts/clean`, then `--yes` (log scratch in `.work/.task-created`).

## Colin McRae Rally 04: rules and decisions

- **One binary patch.** The only patch is the community widescreen fix in
  `src/edition.rs`, a `rea_launcher::patch::PatchSet` applied from pristine
  bytes to builds identified by SHA-256, with its result hash. An unknown
  build runs unpatched and the player is asked to report its SHA-256
  (issue form "Unknown edition"). A new edition goes into the table only
  with its digest and the patched digest measured on it.
- **The resolution decides widescreen** (owner decision, 2026-11-02): the
  fix applies to 16:9 resolutions only, and there is no separate switch
  (it overlapped the resolution). Old `widescreen` lines and flags are
  accepted and ignored.
- **The player's own DLLs stay.** Preserve game-owned runtime data across
  source syncs, and never overwrite a DLL the player put in the game
  folder (a DSOAL `dsound.dll`) just to install a launcher component.
- **dgVoodoo2 2.80 is the one hosted payload** (owner decision, recorded
  2026-11-02): `dgVoodoo2_80.zip`, holding only the tested
  `MS/x86/D3D9.dll`, lives in this repository's `thirdparty` release (pin
  `dgvoodoo` in `packaging/components.pins`), never in Git; see
  `packaging/mirror/README.md`. No other licence-less payload is allowed;
  the official 2.80 archive is gone upstream and 2.87.6 crashed CMR04.
- **One supported installer** (owner decision, 2026-11-09): Magipack's
  repack, unpacked with the bundled innoextract into `<runtime>-game` and
  never run (`src/source.rs`, sdk `docs/playbooks/installers.md`). A new
  installer goes into `source::INSTALLERS` only after its unpacked game
  was compared with an installed copy, with its `prefer`/`drop` lists
  (`docs/RESEARCH.md`, "Installers"). After an unpack the saved `game`
  points at the unpacked folder, except when the folder came from `--game`.
- **Packages** are built only in CI output folders or `.work/`, never the
  repository root, and the package check (`packaging/package.allow`)
  rejects unlisted files and every game asset.
