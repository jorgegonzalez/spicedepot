# Releasing SpiceDepot

Releases are built and published by GitHub Actions when a version tag is pushed. The release workflow builds macOS universal, Linux x64, and Windows x64 packages, publishes them to GitHub Releases, then updates the Homebrew tap.

## Prereqs (one-time setup)

### 1. Homebrew tap PAT

The tap lives at [jorgegonzalez/homebrew-tap](https://github.com/jorgegonzalez/homebrew-tap). The `update-tap.yml` workflow in this repo needs a fine-grained personal access token to push commits there.

Generate at <https://github.com/settings/personal-access-tokens/new>:

- **Resource owner**: `jorgegonzalez`
- **Repository access**: Only select repositories → `homebrew-tap`
- **Permissions** → Repository permissions → **Contents: Read and write**
- **Expiration**: 1 year (set a calendar reminder to renew)

Add to this repo as the secret `HOMEBREW_TAP_TOKEN`:

```bash
gh secret set HOMEBREW_TAP_TOKEN --repo jorgegonzalez/spicedepot
# paste the PAT
```

### 2. (Optional, recommended) Apple signing + notarization

Without these, the bundled `.dmg` works but macOS Gatekeeper shows a warning on first launch and `homebrew-cask` (the official tap) will reject the cask. Our own tap accepts it either way.

When you're ready, follow Tauri's [signing guide](https://v2.tauri.app/distribute/sign/macos/) and add these secrets to this repo:

- `APPLE_CERTIFICATE` — base64 of your Developer ID Application `.p12`
- `APPLE_CERTIFICATE_PASSWORD` — password used when exporting the `.p12`
- `APPLE_SIGNING_IDENTITY` — e.g. `Developer ID Application: Jorge Gonzalez (TEAMID)`
- `APPLE_ID` — your Apple ID email
- `APPLE_PASSWORD` — app-specific password for `notarytool`
- `APPLE_TEAM_ID` — 10-char team ID

`tauri-action` picks these up automatically when present.

## Cutting a release

```bash
# Bump the version in two places (must match):
#   src-tauri/Cargo.toml    → `version = "X.Y.Z"`
#   src-tauri/tauri.conf.json → `"version": "X.Y.Z"`
# (frontend package.json is also bumped but doesn't gate the build)

git commit -am "Bump to vX.Y.Z"
git tag vX.Y.Z
git push origin main vX.Y.Z
```

That's it. The pipeline:

1. **`.github/workflows/release.yml`** triggers on the tag push. Matrix builds for macOS universal (Apple Silicon + Intel), Linux x64, Windows x64. Uploads artifacts to a GitHub Release titled `SpiceDepot vX.Y.Z`. Since `releaseDraft: false`, the release auto-publishes.
2. **`.github/workflows/update-tap.yml`** triggers on the `release: published` event. Downloads `SpiceDepot_X.Y.Z_universal.dmg`, computes SHA256, rewrites the cask in `jorgegonzalez/homebrew-tap`, pushes.
3. End-users can run `brew install --cask jorgegonzalez/tap/spicedepot` (or `brew upgrade --cask spicedepot` if they already had it).

Total time tag-to-installable: ~25 minutes (most of it in the cross-platform matrix build).

## Verify a release worked

```bash
# Tap install works
brew tap jorgegonzalez/tap
brew install --cask spicedepot
open -a SpiceDepot

# Or upgrade an existing install
brew upgrade --cask spicedepot
```

If `brew install` reports an old version, the `update-tap.yml` run probably failed. Check [Actions on the tap repo](https://github.com/jorgegonzalez/homebrew-tap/commits/main) — every release should produce a "spicedepot vX.Y.Z" commit there.

## Manually re-sync the tap

If a tap update gets stuck or out of sync, kick it off by hand from the Actions tab:

> Actions → Update Homebrew tap → Run workflow → tag `vX.Y.Z`

The workflow's `workflow_dispatch` input takes any existing release tag and re-runs the SHA256 compute + tap push, no re-build needed.

## Promoting to `homebrew-cask` (official)

Once releases are signed + notarized (see Apple section above), you can submit the cask to [homebrew/homebrew-cask](https://github.com/Homebrew/homebrew-cask) so users get `brew install --cask spicedepot` without the tap step:

1. Fork `homebrew/homebrew-cask`
2. Copy `Casks/spicedepot.rb` from our tap into `Casks/s/spicedepot.rb` (note: subdirectory by first letter)
3. Run `brew audit --new spicedepot` locally; fix anything it flags
4. Open a PR. Expect review feedback; iterate.

After acceptance, drop our tap from the install instructions and just say `brew install --cask spicedepot`.
