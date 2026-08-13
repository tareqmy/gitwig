---
name: rust-quality
description: Runs Gitwig's mandatory quality gates — cargo fmt, clippy with -D warnings and -D clippy::unwrap_used, and the test suite — before committing Rust changes. Trigger before committing anything under src/ or gitwig-core/.
---

# Rust Quality Checks

Gitwig enforces strict formatting, linting, and test-passing standards before any commit.
These are the same checks CI runs (`.github/workflows/ci.yml` → `make fmt-check`,
`make lint`, `make test`), so passing locally means CI won't fail on style or lint grounds.

Mirrors `.agent/skills/rust-quality/SKILL.md` (read by other agents) — keep both in sync
if the process changes.

## Process

1. **Format** — `make fmt` (applies `cargo fmt`). CI itself only *checks* via
   `make fmt-check`, so run the applying form here.
2. **Lint** — `make lint`, i.e. `cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used`.
   The `unwrap_used` deny is deliberate, not incidental: see the "No Panics in Hot Paths"
   rule in `.agent/STYLE_GUIDE.md` — `.unwrap()`/`.expect()` in rendering or the event loop
   are lint errors here, not style nits. Fix the underlying code; don't paper over it with
   `#[allow(...)]` unless a case is a genuine false positive.
3. **Test** — `make test` (`cargo test` across the workspace). All tests must pass.
   - Any new feature, action, or popup needs comprehensive tests in `src/app/tests.rs` or
     `src/ui/draw.rs` (headless rendering or temp Git repos).
   - Never land a change that drops overall test coverage.

`make ci` runs fmt-check + lint + test in one shot — a convenient final check, but run
`make fmt` first since `ci` only checks formatting rather than applying it.

## Post-run
If any step fails or warns, fix the underlying code and re-run before proceeding. Don't
skip or suppress a gate just to get to green.
