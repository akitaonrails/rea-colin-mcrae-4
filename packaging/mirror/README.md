# dgVoodoo2 2.80: the one archive we host

`dgVoodoo2_80.zip` contains only `MS/x86/D3D9.dll`, the dgVoodoo2 2.80
compatibility DLL used by the working CMR04 recipe. It is not a Codemasters
asset and contains no game data.

- DLL SHA-256: `6f0eae6d0fbe836490b1aa3361304d2398fdb5d815a929e126b5ae7b13b16e47`
- ZIP SHA-256: `cec7e2c8eeaec0065cc8f41d8e0f176ae28d55074c4295ce3670589dc9f29d30`
- Official project: https://dege.freeweb.hu/dgVoodoo2/dgVoodoo2/
- Official repository: https://github.com/dege-diosg/dgVoodoo2
- Copyright: Dege

Controlled CMR04 tests found that 2.87.6 still crashed even with registry
paths and forced D3D11. The tested 2.80 DLL plus DXVK reached the 4K title and
3D attract demo after install paths were seeded. Dege's v2.8 release exposes
only the unrelated dgVoodoo API archive, not the full 2.80 binary distribution.

dgVoodoo2 ships no licence text. Hosting this one DLL is the owner's
recorded exception (`THIRD_PARTY.md`, `AGENTS.md`). Where it lives:

- **Not in Git.** Until 2026-11-02 the zip was committed here
  (`packaging/mirror/dgVoodoo2_80.zip`); it was removed going forward and
  history was not rewritten.
- **In the repository's `thirdparty` release**, pinned by SHA-256 in
  `packaging/components.pins` (`dgvoodoo`) and fetched at package time by
  `packaging/fetch-components.sh`.

The owner creates that release once (not as "latest") from the last commit
that had the file:

```sh
git show 0d1eeb4:packaging/mirror/dgVoodoo2_80.zip > /tmp/dgVoodoo2_80.zip
sha256sum /tmp/dgVoodoo2_80.zip   # cec7e2c8…9d30
gh release create thirdparty --latest=false --title "Third-party files" \
  --notes "Pinned third-party files the launcher packages (see packaging/mirror/README.md)." \
  /tmp/dgVoodoo2_80.zip
```

This file is also the dgVoodoo notice included in packages.
