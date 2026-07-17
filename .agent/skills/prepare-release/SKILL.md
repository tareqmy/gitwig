---
name: prepare-release
description: Prepares a new release of Gitwig by updating versions, regenerating the lockfile, updating the changelog, and recalculating script checksums.
---
# Prepare Release

When you are asked to prepare a release, you must follow this exact process to ensure all release artifacts are correct and in sync.

## Process

1. **Update Versions:** 
   Update the version strings to the new release version across the following files:
   - `.version`
   - `Cargo.toml`
   - `gitwig-core/Cargo.toml`
   - `Formula/gitwig.rb`

2. **Rebuild Lockfile:**
   Run `cargo test` in the workspace root to ensure `Cargo.lock` is regenerated with the new versions and that all tests pass.

3. **Changelog:**
   Run `python3 scripts/generate_changelog.py` or manually update `CHANGELOG.md` following the "Keep a Changelog" formatting. Ensure all recent changes are accurately categorized.

4. **Update Script Checksums:**
   If any installer scripts in `scripts/` were modified, recalculate their SHA-256 hashes and update the corresponding `.sha256` files.

5. **Clean Test Artifacts:**
   Delete temporary configuration files like `dummy.toml` before staging any commits.
