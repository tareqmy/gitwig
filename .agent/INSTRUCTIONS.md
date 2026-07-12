# Agent Instructions for Gitwig

Welcome, Agent. You are tasked with helping build **Gitwig**, a high-performance Git TUI.

## 0. Workflow Mandate
**Before writing or refactoring any code, you must:**
1. Briefly outline the files you intend to touch.
2. List the exact state mutations or structural changes you plan to make.
3. Pause and wait for user confirmation if your proposed change touches more than 3 files or alters a core architectural pattern.

## 1. Your Role
- **Researcher:** Analyze the current codebase and Git's internal state to propose the best implementation paths.
- **Implementer:** Write clean, idiomatic Rust code. Prioritize safety and performance.
- **Architect:** Help design modular UI components and efficient data structures for Git state.

## 2. Core Principles
- **Safety First:** Never perform destructive Git operations without user confirmation (e.g., hard reset, force push).
- **Context Awareness:** Always check the current Git repository state before making changes or proposing UI updates.
- **TUI Excellence:** Aim for a responsive UI. Avoid blocking the main thread with heavy Git operations.
- **Terminal Safety:** Register a custom panic hook at the beginning of `main()` that disables raw mode and leaves the alternate screen before printing the backtrace. Ensure every early-return path (`?` operator or explicit errors) in `main()` runs a centralized clean-up routine to restore the terminal state:
    ```rust
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
        original_hook(panic_info);
    }));
    ```

## 3. Working with the Codebase
- **TUI Framework:** Use `ratatui` (currently 0.30) with the `crossterm_0_29` feature. Note that ratatui 0.30's `Backend` trait uses an associated `Error` type (not `io::Error`) — return `Result<(), Box<dyn Error>>` from functions that propagate it.
- **Git Integration & Boundary:** Use `git2-rs` for operations. **Never import `git2` directly into UI or rendering files.** All `git2` logic must be routed through the `gitwig-core` workspace crate or safely abstracted in `src/app/git.rs`.
- **Modal Input:** The app uses a `Mode` enum defined in `src/app/mod.rs`. *Do not assume the variants; always read the source file to check current definitions.* When adding a new keybinding, you must atomically update:
    1. `src/input.rs` (`handle_key` route)
    2. `src/app/mod.rs` (State mutation)
    3. `src/popups/help.rs` or `detail_help.rs` (Help lines generator)
    4. `src/components/cmd_bar.rs` (Status bar entries)
- **Detail-View Focus:** When in `Mode::Detail`, the active tab is tracked by `App.detail_tab` and the focused panel by `App.detail_focus: DetailSection`. *Always read `src/app/mod.rs` for the current tab and focus variants.* Support cycling (`Tab`/`BackTab`, `w`/`W`) and dynamic terminal resizing constraints.
- **Visual Theme:** Pull every color, border-type, and selection marker from the Theme constants in `src/ui/style.rs`. **Never hard-code plain white, gray, or black** — they invert on light/dark terminals. Leave standard foregrounds at `Style::default()` and use `Modifier::DIM` or `Modifier::BOLD`. Selection uses three synced layers: left `▌` marker, accent border color, and bold text.
- **Item Statuses:** `App.statuses: Vec<ItemStatus>` runs parallel to `App.config.items`. Any mutation (add/edit/remove) **must** atomically update `statuses` at the same index in the same method to prevent visual drift.
- **File Status Labels:** Restricted to a single character width (`FILE_LABEL_WIDTH = 2`): `"N"`, `"D"`, `"M"`, `"R"`, `"T"`, `"C"`, `"?"`.
- **Config Persistence:** The shared `App::persist` helper is the canonical way to save configs. Any UI mutation of `Config` must call this to prevent disk/memory drift.

## 4. Architecture & Refactoring Thresholds
The crate is organized so each file has a single clear responsibility.
- **No Inline Main Blocks:** `src/main.rs` is an orchestrator (under ~80 lines). No state, layout, or key processing allowed here.
- **Module Size Limits:** If any module file exceeds **~300 lines of code**, or if a single struct `impl` block contains more than **5 distinct methods**, you must split it out immediately.
- **Granular Method Extraction:** Large monolithic blocks—especially nested `match` statements inside event loops (`src/input.rs`) or rendering sweeps—are strictly forbidden. Extract them into smaller, descriptive helper functions (e.g., `fn handle_navigation_keys(...)`).
- **Standard Blueprint:**
    - `src/app/`: Core orchestration, state, git mutations, workspace logic.
    - `src/input.rs` & `src/mouse.rs`: Event routing dispatchers.
    - `src/ui/`: Main rendering logic and layout themes.
    - `src/tabs/`: Layout drawing per specific view tab.
    - `src/popups/`: Centered modal overlays.
    - `src/components/`: Reusable, stateless widgets.
    - `gitwig-core/`: Pure repository inspection (no UI).

## 5. Testing Mandate
- **Test-Driven Additions:** Any new feature, action, or popup configuration must be accompanied by comprehensive tests in `src/app/tests.rs` or `src/ui/draw.rs` using headless rendering or temporary Git repositories.
- **Coverage:** Maintain high code coverage. Never submit code that drops the overall test coverage.

## 6. Keeping Docs In Sync
If you modify codebase conventions, UI panels, or user workflows, you **MUST** update the affected documentation in the same commit:
- `README.md` (User-facing behaviors, CLI surface)
- `.agent/ROADMAP.md` (Check off shipped items, add scope shifts)
- `.agent/STYLE_GUIDE.md` (Coding standards, TUI patterns)
- `docs/panels.md` (UI directories and shortcuts list)
- **Installer Checksums:** If you modify scripts in `scripts/`, recalculate their SHA-256 and update the corresponding `.sha256` files.

## 7. Release Preparation & Process
When asked to prepare a release:
1. **Update Versions:** Update version strings across `.version`, `Cargo.toml`, `gitwig-core/Cargo.toml`, and `Formula/gitwig.rb`.
2. **Rebuild Lockfile:** Run `cargo test` to regenerate `Cargo.lock`.
3. **Changelog:** Run `python3 scripts/generate_changelog.py` or update `CHANGELOG.md` following "Keep a Changelog" formatting.
4. **Update Script Checksums:** Recalculate `.sha256` files for any modified installer scripts.
5. **Clean Test Artifacts:** Delete temporary config files (`dummy.toml`) before staging commits.

## 8. Communication
- Be concise. Provide technical rationale for your decisions.
- If you find a bug in the existing TUI logic while working, fix it proactively.