---
name: prepare-release
description: Prepares a new Gitwig release — bumps version strings across every file that carries one, regenerates the lockfile, updates the changelog, and recalculates installer script checksums. Trigger when asked to cut, prepare, bump, or tag a release.
---

# Prepare Release

Follow this exact process so every release artifact stays in sync. Mirrors
`.agent/skills/prepare-release/SKILL.md`.

## Process

1. **Update versions** — bump the version string in every file that carries one (verified
   against the current release, `2.5.10`):
   - `.version`
   - `Cargo.toml`
   - `gitwig-core/Cargo.toml`
   - `Formula/gitwig.rb`
   - `dist/chocolatey/gitwig.nuspec`
   - `dist/chocolatey/tools/chocolateyinstall.ps1`

   Sanity-check nothing was missed with:
   `grep -rl "<old-version>" . --include="*" | grep -vE "target/|\.git/|Cargo\.lock|CHANGELOG\.md"`

2. **Rebuild the lockfile** — run `cargo test` at the workspace root so `Cargo.lock`
   regenerates with the new versions and the suite still passes.

3. **Changelog** — run `python3 scripts/generate_changelog.py`, or update `CHANGELOG.md`
   by hand following "Keep a Changelog" formatting. When cutting a release, move the
   accumulated changes out of "Unreleased" into a section for the new version — the
   commit that follows gets tagged with that version, so "Unreleased" must be empty
   afterward.

4. **Script checksums** — if any installer script under `scripts/` changed, recalculate
   its SHA-256 (`shasum -a 256 <script>`) and update the matching `.sha256` file.

5. **Clean test artifacts** — delete temporary config files (e.g. `dummy.toml`) created by
   manual testing before staging the release commit.

## Before committing
Run the `rust-quality` skill's checks — a release commit is still a commit.
