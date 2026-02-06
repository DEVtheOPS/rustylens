# Homebrew Distribution

This project can publish a cask to a custom Homebrew tap on each GitHub release tag.

## 1) Create a tap repo

Create a repository like:

- `DEVtheOPS/homebrew-kore`

Expected cask path:

- `Casks/kore.rb`

## 2) Configure this repo

In this repo's GitHub settings, set:

- Repository variable: `HOMEBREW_TAP_REPO`
  - Example: `DEVtheOPS/homebrew-kore`
- Repository secret: `HOMEBREW_TAP_TOKEN`
  - Fine-grained PAT with `Contents: Read and write` on the tap repo.

Workflow file:

- `.github/workflows/homebrew-tap.yml`

It triggers when a release is published (`v*` tags from the release workflow).

## 3) User install command

Without Apple notarization, quarantine can block launch. Prefer:

```bash
brew install --cask --no-quarantine devtheops/kore/kore
```

If already installed and blocked:

```bash
xattr -dr com.apple.quarantine /Applications/Kore.app
```

## Notes

- The workflow picks a `.dmg` asset from the release (prefers `universal` if present).
- It computes `sha256` and updates `Casks/kore.rb` automatically.
