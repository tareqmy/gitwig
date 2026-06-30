# Gitwig Audit Remediation Plan

Based on three independent audit documents in `.agent/audits/`:
- `SECURITY_AND_FUNCTIONAL_AUDIT.pdf`
- `gitwig-Security-Functional-Assessment.pdf`
- `gitwig-Audit-Summary.pdf`

---

## Overall Verdict

| Domain | Grade | Summary |
|---|---|---|
| Security | **B-** | Strong hygiene, but fixable systemic injection class + one realistic RCE |
| Functional/Quality | **C+** | Core works, but dead conflict-resolution workflow, broken lint gate, data-loss config path |

No backdoors found. All issues are ordinary defects.

---

## P0 — Fix Before Next Release (Critical/High)

### S2 — RCE via `ext::` remote URL on fetch/pull
- **Severity:** High (CWE-78, realistic RCE)
- **Problem:** Network git calls (`fetch`, `pull`, `clone`, `ls-remote`) do not restrict `GIT_ALLOW_PROTOCOL`. A malicious repo with `url = ext::sh -c '<cmd>'` in `.git/config` → full RCE when user triggers fetch.
- **Fix:** On every network `git` call, add `.env("GIT_ALLOW_PROTOCOL", "https:ssh:git:file")` and `.env("GIT_PROTOCOL_FROM_USER", "0")`, or pass `-c protocol.ext.allow=never`. Reject `ext::` / `fd::` in clone/import dialogs.
- **Files:** `src/app/git.rs:36` (pull), `:1036` (fetch), `gitwig-core/lib.rs:2108` (ls-remote), `src/app/navigation.rs:2132` (clone)

---

### S1 — Argument injection: repo-derived refs/paths without `--`
- **Severity:** High (CWE-88)
- **Problem:** ~40 git invocations pass branch/tag/remote/file names positionally with no `--` end-of-options separator. A tag named `--points-at` or a conflicted file named `-R` causes git to misparse it as an option.
- **Fix:** Insert `.arg("--")` before every repo/user-derived positional ref/path. Create a centralized `safe_ref(&str) -> Result<&str>` guard that rejects leading `-`. Route all call sites through it.
- **Representative sites:**
  - `gitwig-core/lib.rs:1961` `git checkout <branch>`, `:2086` `git checkout <tag>`
  - `src/app/git.rs:726` `git merge <target>`, `:821` `git rebase <target>`, `:1039` `git fetch <remote>`

---

### R1 — Non-atomic config save → data loss + startup lockout
- **Severity:** High (Reliability)
- **Problem:** Config save does truncate-then-write (`config.rs:599`). A crash mid-write → empty/partial file = loss of all items, labels, pinned, visits. A malformed config on load causes `toml::from_str(...)?` to propagate up to `main.rs:113` after the alternate screen is entered → app exits and refuses to open. The `.bak` is only written on version change so it may be stale.
- **Fix:**
  - Atomic write: write to `config.toml.tmp` then `rename()`.
  - On parse error: rename the bad file to `config.toml.corrupt-<ts>` and start from defaults with a warning toast.
- **Files:** `src/config.rs:599` (save), `:403` (load)

---

### F15 — Conflict-panel `A` (abort) and `C` (continue) keys are dead
- **Severity:** High (core workflow broken)
- **Problem:** In `src/tabs/workspace.rs`, the generic `'a' | 'A' if is_uncommitted_selected()` arm at `:249` and `'C' if is_uncommitted_selected()` arm at `:280` are evaluated before the Conflicts-specific arms at `:305` (`A → MergeAbortConfirm`) and `:312` (`C → MergeContinueConfirm`). Rust matches top-down → conflict abort/continue are unreachable.
- **Fix:** Reorder match arms so Conflicts-specific arms precede the generic stage/commit arms, or tighten guards to exclude the `is_conflict_panel_focused()` case from the generic arms.
- **Files:** `src/tabs/workspace.rs:249`, `:280`, `:305`, `:312`

---

### F01 — Hidden hard dependency on system `git` binary; no preflight
- **Severity:** High (UX/Functional)
- **Problem:** Network ops and local ops (`add -A`, `reset`, `checkout -- .`, `clean -fd`, `apply`, `log`) require `git` on PATH, but there is no startup probe. Failures surface as opaque errors at first use.
- **Fix:** Probe for `git` (and optionally `fzf`, `ssh`) at startup. Surface a clear one-time warning toast. Document runtime dependencies in README/INSTALL.
- **Files:** `src/main.rs` (add startup probe), `README.md`

---

## P1 — Fix Soon (Medium)

### S3 — Command injection via `fzf.excludes` in `sh -c`
- **Severity:** High severity / Medium likelihood (CWE-78)
- **Problem:** The fzf command is assembled as a shell string via `sh -c`. `start_dir` is single-quote-escaped, but each `excludes` entry is interpolated raw: `format!("--exclude '{}'", x)`. A value `x';id;'` executes arbitrary shell. Additionally, `load_config` migrates `./config/config.toml` relative to CWD on first run — launching inside a malicious repo auto-imports hostile excludes.
- **Fix:** Stop using `sh -c`; build fzf command as a `Command` with `.arg()` array. Or: escape every interpolated value with a proper shell-escape function. Treat CWD-migrated config as untrusted.
- **Files:** `src/app/mod.rs:1328`, `:1336`, `:1347`, `:1366`, `:1423`, `:1431`

---

### S4 — Clipboard / fzf-stdin / child stdio bypass ratatui's control-char filter
- **Severity:** Medium (terminal escape injection)
- **Problem:** ratatui strips control chars at the buffer level, but clipboard yanks, fzf stdin, and child stdio bypass ratatui's filter entirely.
- **Fix:** Add a `sanitize_for_terminal(s: &str) -> String` function that strips OSC/CSI/other escape sequences. Apply before writing to clipboard, fzf stdin, and any non-ratatui output path.

---

### F04 — Destructive confirmation dialogs default `Enter` to Yes
- **Severity:** Medium
- **Problem:** `ConfirmPopup` maps `'y' | 'Y' | Enter → ConfirmYes` (`src/popups/confirm.rs:1017`). A reflexive Enter after Delete Branch/Tag/Stash/Discard/Remote-Delete confirms destruction. The Home delete-confirm requires explicit `y` — inconsistent.
- **Fix:** Change default Enter to ConfirmNo for all destructive dialogs. Require explicit `y` to confirm. Consider typed-name confirmation for Discard-All / Abort-Merge.
- **Files:** `src/popups/confirm.rs:1017`

---

### F05 — Discard-All is one Enter away from irreversible loss
- **Severity:** Medium (direct consequence of F04)
- **Problem:** `X` → `git reset + git checkout -- . + git clean -fd` deletes untracked files. Gating exists (`Mode::DiscardChangesConfirm`) but combined with F04 (Enter=Yes) this is the most dangerous one-keystroke path.
- **Fix:** Address via F04 fix. Additionally consider a typed confirmation (e.g. type `"discard"`) for Discard-All specifically.

---

### Q2 — Lint gate broken: `cargo clippy -D warnings` fails
- **Severity:** Medium (CI broken)
- **Problem:** `unnecessary_map_or` lint at `src/app/navigation.rs:31`. Because the crate has `#![deny(clippy::all)]`, this is a hard stop under Rust 1.96. CI lint is currently red even though `cargo build` passes.
- **Fix:** Change `map_or(false, …)` → `is_some_and(…)` at the offending site.
- **Files:** `src/app/navigation.rs:31`

---

## P2 — Hardening / Polish (Low / Info)

### S5 — Config/log file permissions
- **Fix:** Set `0600` permissions on `config.toml` and `0700` on the config directory on Unix after creation.

### S7 — SHA slice not char-safe
- **Fix:** Use `&sha[..8.min(sha.len())]` with a `.is_char_boundary()` check when displaying abbreviated SHA to avoid a potential panic on non-ASCII (though unlikely in practice).

### S8 — Patch path hardening
- **Fix:** Validate that paths embedded in manually-built patch hunks do not escape the repository root.

### S9 — Log rotation
- **Fix:** Add log file rotation or a maximum size cap for the debug log file.

### S6 — `accept-new` SSH host-key policy
- **Fix:** Document or scope the `accept-new` policy; don't silently accept arbitrary new host keys in non-interactive contexts.

### F03 — "Delete remote branch" only deletes local tracking ref
- **Severity:** Medium (misleading behavior, no data loss)
- **Problem:** `delete_remote_branch` at `gitwig-core/lib.rs:2037` only calls `find_branch(Remote).delete()` — the server branch is untouched and reappears on next fetch. Toast reads "Deleted branch ''" — misleading.
- **Fix:** Either shell out `git push <remote> --delete <branch>` (with confirmation), or change the toast to clearly say "Removed local tracking ref only".

### F12 — Commit-target index mis-points under active search + dirty tree
- **Severity:** Medium (SUSPECTED)
- **Problem:** Action sites recompute `<uncommitted>` row offset with `selection.saturating_sub(1)` independently of `get_selected_commit`, which also factors in `search_query`. With a filtered commit search, offsets diverge → tag/cherry-pick/revert may target wrong commit.
- **Fix:** Route all action sites through `get_selected_commit()`; never recompute the dirty offset locally.
- **Files:** `src/app/git.rs:243`, `src/app/workspace.rs:14/146`, `src/app/navigation.rs:1486`

### F09/F10/F11 — Background result attribution / stash re-validation
- **Problem:**
  - F10: `dismiss_fetch` leaves child running; its later message is applied to whichever repo is now selected.
  - F11: Stash delete/apply capture numeric stash index at confirm time; a file-watcher refresh between request and confirm could drop the wrong stash.
- **Fix:** Attach repo identity to background messages (F10). Re-validate stash by name/message at confirm time (F11).

### F08 — Theme load hard-fails in canonical path, soft-fails in CLI path
- **Fix:** Make both theme load paths tolerant; fall back to the default theme with a warning instead of panicking/exiting.

### F13 / F17 — UX consistency for dead entries and confirm defaults
- **Fix:** Add a clear "Remove dead entry" affordance for non-git paths. Align confirm-default behavior between Home and Detail views.

### Q1 — Test suite fails on Windows: EOL-fragile patch construction (tracked for awareness)
- **Problem:** Manually-built patches always use `\n`; CRLF-checked-out files on Windows cause 4/15 core tests to fail, and real hunk/line staging may mis-apply on CRLF repos.
- **Fix:** Preserve original EOL when constructing patches (or apply via libgit2). Make tests set `core.autocrlf=false` on their fixtures. (Lower priority as the app targets macOS/Linux primarily.)

---

## Suggested Implementation Order

| Step | Finding(s) | Effort |
|---|---|---|
| 1 | **Q2** — Fix `map_or` → `is_some_and` | ~5 min |
| 2 | **F15** — Reorder workspace.rs match arms | ~30 min |
| 3 | **R1** — Atomic config save + parse-error recovery | ~1-2 hrs |
| 4 | **F01** — Startup git probe + README update | ~1 hr |
| 5 | **S2** — Add `GIT_ALLOW_PROTOCOL` env to all network git calls | ~1 hr |
| 6 | **S1** — Add `--` separator + `safe_ref()` guard to all git calls | ~2-3 hrs |
| 7 | **F04/F05** — Fix Enter=No default for destructive dialogs | ~1 hr |
| 8 | **S3** — Fix fzf shell injection (use `Command::arg` array) | ~1 hr |
| 9 | **S4** — Add `sanitize_for_terminal` helper | ~30 min |
| 10 | **F03** — Fix remote branch delete (actually push delete) | ~1 hr |
| 11 | **F12** — Unify commit index via `get_selected_commit()` | ~1 hr |
| 12 | P2 polish items (S5, S7, S8, S9, F08, F10, F11, F13, F17) | Ongoing |
