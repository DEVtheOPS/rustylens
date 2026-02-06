# Winget Release Checklist

Use this checklist for every Windows release.

## Pre-release

- [ ] `package.json` version matches release version.
- [ ] `src-tauri/Cargo.toml` version matches release version.
- [ ] `src-tauri/tauri.conf.json` version matches release version.
- [ ] Windows installer artifacts are present in GitHub Release.

## Gather release metadata

- [ ] Copy final installer URL (e.g. `.exe` or `.msi`).
- [ ] Compute SHA256 of installer.
- [ ] Confirm installer type (`nullsoft` for NSIS `.exe`, `wix` for MSI).
- [ ] Confirm silent install switches work.

## Manifest generation

- [ ] Preferred: run `wingetcreate update DEVtheOPS.Kore --version <VERSION> --urls <URL>`.
- [ ] Fallback: fill templates in `packaging/winget/templates/`.

## Validation

- [ ] Run Winget manifest validation.
- [ ] Ensure package identifier is `DEVtheOPS.Kore`.
- [ ] Ensure version and SHA256 match release exactly.

## Publish

- [ ] Submit PR to `microsoft/winget-pkgs`.
- [ ] Watch CI checks until green.
- [ ] Merge (or wait for maintainer merge).

## Post-publish

- [ ] Verify install:

```powershell
winget install --id DEVtheOPS.Kore -e
```

- [ ] Verify upgrade:

```powershell
winget upgrade --id DEVtheOPS.Kore -e
```
