---
name: prepare-release
description: Prepares a new Gitwig release — bumps version strings across every file that carries one, regenerates the lockfile, updates the changelog, and recalculates installer script checksums. Trigger when asked to cut, prepare, bump, or tag a release.
---
# Prepare Release

_Mirrored for Claude Code at `.claude/skills/prepare-release/SKILL.md` — keep both in sync._

When you are asked to prepare a release, follow this exact process so every release artifact stays in sync.

## Process

1. **Update Versions:**
   Bump the version string in every file that carries one (the current release is the
   version in `.version`):
   - `.version`
   - `Cargo.toml`
   - `gitwig-core/Cargo.toml`
   - `Formula/gitwig.rb`
   - `dist/chocolatey/gitwig.nuspec`
   - `dist/chocolatey/tools/chocolateyinstall.ps1`

   Sanity-check nothing was missed with:
   `grep -rl "<old-version>" . --include="*" | grep -vE "target/|\.git/|Cargo\.lock|CHANGELOG\.md"`

2. **Rebuild Lockfile:**
   Run `cargo test` in the workspace root so `Cargo.lock` regenerates with the new versions and the suite still passes.

3. **Changelog:**
   Run `python3 scripts/generate_changelog.py` or manually update `CHANGELOG.md` following the "Keep a Changelog" formatting. Ensure all recent changes are accurately categorized. When cutting a release, move the accumulated changes out of "Unreleased" into a section for the new version — the commit that follows gets tagged with that version, so "Unreleased" must be empty afterward.

4. **Update Script Checksums:**
   If any installer scripts in `scripts/` were modified, recalculate their SHA-256 hashes (`shasum -a 256 <script>`) and update the corresponding `.sha256` files.

5. **Clean Test Artifacts:**
   Delete temporary configuration files like `dummy.toml` created by manual testing before staging any commits.

## Before Committing
Run the `rust-quality` skill's checks — a release commit is still a commit.
