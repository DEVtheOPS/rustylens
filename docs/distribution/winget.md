# Winget Distribution

This project can be distributed on Windows via the official Winget community repository:

- `https://github.com/microsoft/winget-pkgs`

Package ID used by templates in this repo:

- `DEVtheOPS.Kore`

## Recommended release flow

1. Publish a new GitHub release with Windows installer assets.
2. Compute installer SHA256 hashes.
3. Generate/update Winget manifests.
4. Submit manifest PR to `microsoft/winget-pkgs`.

## Fast path with wingetcreate (recommended)

Install:

```powershell
winget install Microsoft.WingetCreate
```

Create/update from release URL:

```powershell
wingetcreate update DEVtheOPS.Kore `
  --version <VERSION> `
  --urls <WINDOWS_INSTALLER_URL>
```

Submit PR:

```powershell
wingetcreate submit `
  --manifest-dir <PATH_TO_GENERATED_MANIFESTS> `
  --token <GITHUB_TOKEN_WITH_FORK_ACCESS>
```

## Manual fallback

Use templates in:

- `packaging/winget/templates/`

Fill in:

- `__VERSION__`
- `__INSTALLER_URL__`
- `__INSTALLER_SHA256__`

Then submit those manifest files in a PR to `microsoft/winget-pkgs`.

## Release checklist

See:

- `packaging/winget/release-checklist.md`
