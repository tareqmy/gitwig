---
name: rust-quality
description: Runs Gitwig's mandatory code quality checks (formatting, linting, and testing) before committing code.
---
# Rust Quality Checks

_Mirrored for Claude Code at `.claude/skills/rust-quality/SKILL.md` — keep both in sync._

Gitwig maintains strict performance, safety, and readability standards. Before committing any code, you must pass these quality gates.

## Process

1. **Format Code:**
   Run `cargo fmt` to apply the standard Rust formatting.
   
2. **Lint Code:**
   Run `make lint` (`cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used`) —
   plain `cargo clippy` misses the workspace/all-targets scope and the `unwrap_used` deny
   that CI actually enforces. Fix every warning or error; do not submit code that adds new ones.
   
3. **Run Tests:**
   Run `cargo test` to execute the test suite. All tests must pass.
   - Any new feature, action, or popup must have accompanying comprehensive tests in `src/app/tests.rs` or `src/ui/draw.rs`.
   - Never submit code that drops overall test coverage.

## Post-Run
If any of these commands fail or produce warnings, fix the issues in the code and re-run the checks before proceeding.
