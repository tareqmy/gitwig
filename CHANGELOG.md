# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to Semantic Versioning.

## [Unreleased]
### Added
- expose compact_view layout setting in TUI Settings panel ([266cebd](https://github.com/tareqmy/gitwig/commit/266cebd))
- expose tab_ttl_secs setting in TUI Settings panel ([667d2cd](https://github.com/tareqmy/gitwig/commit/667d2cd))
- expose detail_cache_ttl_secs setting in TUI Settings panel ([47e011b](https://github.com/tareqmy/gitwig/commit/47e011b))
- expose graph_max_commits setting in TUI Settings panel ([72ff63a](https://github.com/tareqmy/gitwig/commit/72ff63a))
- expose enable_commit_signatures setting in TUI Settings panel ([c346b6b](https://github.com/tareqmy/gitwig/commit/c346b6b))
- add show_system_stats configuration to toggle CPU/Memory display in status bar ([4e10ed8](https://github.com/tareqmy/gitwig/commit/4e10ed8))
- show default status symbols 30 seconds after bulk fetch completes ([53752a5](https://github.com/tareqmy/gitwig/commit/53752a5))

### Changed
- organize settings categories and implement word-wrapping for descriptions ([778f996](https://github.com/tareqmy/gitwig/commit/778f996))

### Documentation
- update changelog with exposed settings features ([3037cbc](https://github.com/tareqmy/gitwig/commit/3037cbc))

## [v2.3.19] - 2026-07-04
### Added
- support editing watch directories inside Settings menu ([423854e](https://github.com/tareqmy/gitwig/commit/423854e))
- support mouse scroll, PageUp/PageDown, and Home/End keys in global search results ([62b4528](https://github.com/tareqmy/gitwig/commit/62b4528))
- implement automatic workspace synchronization of newly created or cloned repositories ([fb7a9c5](https://github.com/tareqmy/gitwig/commit/fb7a9c5))
- implement global code search across all tracked repositories ([f08e019](https://github.com/tareqmy/gitwig/commit/f08e019))
- adding changelog ([cc2ec1c](https://github.com/tareqmy/gitwig/commit/cc2ec1c))

### Fixed
- clear generic input_buffer instead of commit_popup buffer and take inputs from status bar for remote add ([52c2c7e](https://github.com/tareqmy/gitwig/commit/52c2c7e))

### Documentation
- update features, keybindings, configuration, and help overlay docs for global search and workspace sync ([4916e6b](https://github.com/tareqmy/gitwig/commit/4916e6b))

## [v2.3.18] - 2026-07-04
### Added
- prompt for labels on adding, bulk adding, and importing/cloning repositories ([140e6a4](https://github.com/tareqmy/gitwig/commit/140e6a4))
- add `gtg` shortcut alias across all installation methods ([746f707](https://github.com/tareqmy/gitwig/commit/746f707))
- allow history graph entries to be selectable and clickable ([d190ba6](https://github.com/tareqmy/gitwig/commit/d190ba6))
- replace implicit tags fetch with scheduled background fetch ([60ec17f](https://github.com/tareqmy/gitwig/commit/60ec17f))

### Fixed
- correct git pull logic and branch push behavior with multiple remotes ([94f771b](https://github.com/tareqmy/gitwig/commit/94f771b))

### Documentation
- split the readme doc ([ecef75a](https://github.com/tareqmy/gitwig/commit/ecef75a))

## [v2.3.17] - 2026-07-03
### Added
- **overview**: add scroll and focus navigation to Overview and Stats panels ([066f8cc](https://github.com/tareqmy/gitwig/commit/066f8cc))
- **files**: add line numbers and git blames to full-screen files view ([7641e72](https://github.com/tareqmy/gitwig/commit/7641e72))

### Changed
- self update should be disabled for app installed through cargo, homebrew and chocolatey ([42bbf4a](https://github.com/tareqmy/gitwig/commit/42bbf4a))

### Others
- show line numbers in full screen content view in files tab ([4366027](https://github.com/tareqmy/gitwig/commit/4366027))

## [v2.3.16] - 2026-07-03
### Testing
- increasing workspace test coverage ([c1c13e0](https://github.com/tareqmy/gitwig/commit/c1c13e0))

## [v2.3.15] - 2026-07-03
### Added
- preparation for chocolatey support ([f7fa3ae](https://github.com/tareqmy/gitwig/commit/f7fa3ae))

### Changed
- detect installed through homebrew and disable in app self update ([63be1ba](https://github.com/tareqmy/gitwig/commit/63be1ba))
- detect installed through cargo and disable in app self update ([3bffc20](https://github.com/tareqmy/gitwig/commit/3bffc20))

### Documentation
- remove private repo instructions because this repo is public ([3ff1d1c](https://github.com/tareqmy/gitwig/commit/3ff1d1c))

### Others
- brew tap needs to enabled for trusting ([9eee970](https://github.com/tareqmy/gitwig/commit/9eee970))

## [v2.3.14] - 2026-07-03
### Added
- remove wix/msi installer support ([33ed118](https://github.com/tareqmy/gitwig/commit/33ed118))

### Changed
- updated shasum ([f4450e8](https://github.com/tareqmy/gitwig/commit/f4450e8))

### Chore
- fix Homebrew tap authentication method in CD workflow ([ff9eb25](https://github.com/tareqmy/gitwig/commit/ff9eb25))

## [v2.3.13] - 2026-07-03
### Others
- solution for publish to homebrew tap ([65a61e1](https://github.com/tareqmy/gitwig/commit/65a61e1))

## [v2.3.12] - 2026-07-03
### Added
- implemented homebrew tap distribution pipeline ([649ee97](https://github.com/tareqmy/gitwig/commit/649ee97))
- plan to support homebrew and chocolatey ([9502aa3](https://github.com/tareqmy/gitwig/commit/9502aa3))
- **discovery**: add native TUI directory scanner fallback for bulk add ([33b3035](https://github.com/tareqmy/gitwig/commit/33b3035))

### Removed
- remove usage of fzf, since we have a builtin fuzzy finder also fzf launching has problems in the windows ([43c5607](https://github.com/tareqmy/gitwig/commit/43c5607))
- remove duplicate keybindings.toml ([06a3dc1](https://github.com/tareqmy/gitwig/commit/06a3dc1))

### Testing
- expand unit test coverage across popups, tabs, routers, and helper modules ([fad1d1e](https://github.com/tareqmy/gitwig/commit/fad1d1e))

## [v2.3.11] - 2026-07-02
### Fixed
- msvc installer build failure correction ([a5aacba](https://github.com/tareqmy/gitwig/commit/a5aacba))

### Changed
- updated keybindings.toml ([347ad73](https://github.com/tareqmy/gitwig/commit/347ad73))

## [v2.3.10] - 2026-07-02
### Testing
- implement headless git repository test generator and expand workspace coverage to 60.9% ([5437084](https://github.com/tareqmy/gitwig/commit/5437084))

## [v2.3.9] - 2026-07-02
### Added
- add fuzzy workspace changes file finder and navigator ([b110862](https://github.com/tareqmy/gitwig/commit/b110862))
- add fuzzy tag checkout panel in tags view ([d684fef](https://github.com/tareqmy/gitwig/commit/d684fef))
- add fuzzy commit search jump panel in history view ([7c30b77](https://github.com/tareqmy/gitwig/commit/7c30b77))
- add native TUI fuzzy file finder fallback ([32ca7cd](https://github.com/tareqmy/gitwig/commit/32ca7cd))
- add fuzzy finder for branches tab ([41332f0](https://github.com/tareqmy/gitwig/commit/41332f0))
- add native TUI repo scan picker and disable default fzf on Windows ([4b35d7c](https://github.com/tareqmy/gitwig/commit/4b35d7c))
- add Windows MSI installer via WiX and customize self-update for MSI builds ([c527f9a](https://github.com/tareqmy/gitwig/commit/c527f9a))

### Fixed
- draw fuzzy search overlays on top of the repository detail view background ([98ccdc4](https://github.com/tareqmy/gitwig/commit/98ccdc4))

### Documentation
- update README shortcuts table to reflect new unified '/' fuzzy search key ([08e07e5](https://github.com/tareqmy/gitwig/commit/08e07e5))
- update detail and general help overlays with new fuzzy finder shortcuts ([3362924](https://github.com/tareqmy/gitwig/commit/3362924))

### Refactored
- remove fuzzy workspace changes search from staged/unstaged changes panels ([c7ec2be](https://github.com/tareqmy/gitwig/commit/c7ec2be))
- unify all fuzzy finder shortcuts to '/' and update all docs, status bars, and tests ([2ebecc7](https://github.com/tareqmy/gitwig/commit/2ebecc7))

### Chore
- bump version to v2.3.9 for release ([a47e915](https://github.com/tareqmy/gitwig/commit/a47e915))

## [v2.3.8] - 2026-07-02
### Added
- design and implement the reflog viewer with direct checkout support ([7eaca2b](https://github.com/tareqmy/gitwig/commit/7eaca2b))
- add interactive mouse-click filtering on global summary bar ([d83165f](https://github.com/tareqmy/gitwig/commit/d83165f))

### Fixed
- fix fzf/fd commands execution on windows by using cmd shell wrapper ([4542b6c](https://github.com/tareqmy/gitwig/commit/4542b6c))
- fix self-update to use install.ps1 via powershell on Windows ([5f2123b](https://github.com/tareqmy/gitwig/commit/5f2123b))

### Changed
- update roadmap to mark reflog viewer as completed ([7362a52](https://github.com/tareqmy/gitwig/commit/7362a52))

### Documentation
- update README.md and INSTRUCTIONS.md for the Reflog tab feature ([72d2a0a](https://github.com/tareqmy/gitwig/commit/72d2a0a))
- optimize README.md title and description block for SEO ([ab42a37](https://github.com/tareqmy/gitwig/commit/ab42a37))

### Chore
- update install script examples and checksums for v2.3.7 release ([33fb072](https://github.com/tareqmy/gitwig/commit/33fb072))

### Others
- wrap custom editor and external git app commands in cmd /c on Windows ([952db36](https://github.com/tareqmy/gitwig/commit/952db36))
- calculate width dynamically for tab headers and fallback to 2 or 1 character headers ([b94be90](https://github.com/tareqmy/gitwig/commit/b94be90))

## [v2.3.7] - 2026-07-02
### Added
- add summary header legends in the signs and symbols legends popup ([0f0b6f2](https://github.com/tareqmy/gitwig/commit/0f0b6f2))

### Changed
- format code and implement polish & UX onboarding panel and fetch spinner ([543df6a](https://github.com/tareqmy/gitwig/commit/543df6a))

### Documentation
- update the docs with signs, symbols and legends and shortcuts ([6ebdc37](https://github.com/tareqmy/gitwig/commit/6ebdc37))

### Chore
- increase short tab name version to 2 characters ([5dbfb83](https://github.com/tareqmy/gitwig/commit/5dbfb83))

### Others
- grouping can be enabled/disabled from settings ([c3a6f12](https://github.com/tareqmy/gitwig/commit/c3a6f12))
- before launching terminal denote the situation and how to get back to the app ([1a35e9f](https://github.com/tareqmy/gitwig/commit/1a35e9f))

## [v2.3.6] - 2026-07-02
### Added
- allow Esc key to cancel multiple selections in normal mode ([ebebb9d](https://github.com/tareqmy/gitwig/commit/ebebb9d))
- dynamically calculate wrapped rows for expanded status bar ([8a1830e](https://github.com/tareqmy/gitwig/commit/8a1830e))
- implement multi-select with Space for batch operations ([c7ed803](https://github.com/tareqmy/gitwig/commit/c7ed803))
- implement bulk fetch (F) with per-card progress indicators ([2f82248](https://github.com/tareqmy/gitwig/commit/2f82248))
- display one-line user-defined note on card from repo settings ([fb11e7e](https://github.com/tareqmy/gitwig/commit/fb11e7e))
- spawn shell/terminal (t) in selected repository ([50b2790](https://github.com/tareqmy/gitwig/commit/50b2790))
- copy repo path to clipboard (y) ([b443993](https://github.com/tareqmy/gitwig/commit/b443993))
- allow ctrl+q to quit application from anywhere in the app ([6e9635b](https://github.com/tareqmy/gitwig/commit/6e9635b))

### Fixed
- align signs and symbols legend popup columns correctly using visual width ([1b8d0ca](https://github.com/tareqmy/gitwig/commit/1b8d0ca))
- wrap and align shortcut descriptions in help popups cleanly ([a8703a4](https://github.com/tareqmy/gitwig/commit/a8703a4))

### Changed
- increase width percentage of help popups to 80% ([d1ae52a](https://github.com/tareqmy/gitwig/commit/d1ae52a))

### Documentation
- update documentation, instructions, and readme to reflect current state of application ([6cc6ce6](https://github.com/tareqmy/gitwig/commit/6cc6ce6))

### Others
- use first character of tab name when space is insufficient ([a8ac1f5](https://github.com/tareqmy/gitwig/commit/a8ac1f5))

## [v2.3.5] - 2026-07-01
### Added
- implement non-blocking background auto-refresh of repository statuses ([e354d4b](https://github.com/tareqmy/gitwig/commit/e354d4b))
- implement uncommitted work warning badge showing PARTIAL when staged & unstaged changes coexist ([ef99ba9](https://github.com/tareqmy/gitwig/commit/ef99ba9))
- implement global summary header bar showing aggregate counts ([7561ee7](https://github.com/tareqmy/gitwig/commit/7561ee7))

### Fixed
- resolve mouse click coordinates and index mapping for repository labels ([8533d7e](https://github.com/tareqmy/gitwig/commit/8533d7e))

### Documentation
- updated the docs and instructions ([0fe8c73](https://github.com/tareqmy/gitwig/commit/0fe8c73))
- mark At-a-Glance Stats as complete in roadmap ([1faf4da](https://github.com/tareqmy/gitwig/commit/1faf4da))

## [v2.3.4] - 2026-07-01
### Removed
- since old versions looking for them here putting it for the time being will remove them in next iteration ([f34bd6e](https://github.com/tareqmy/gitwig/commit/f34bd6e))

## [v2.3.3] - 2026-07-01
### Added
- use variable-height scrolling algorithm for repo jump ([4e3f94b](https://github.com/tareqmy/gitwig/commit/4e3f94b))
- scroll repository into view upon jumping selection ([f4fd038](https://github.com/tareqmy/gitwig/commit/f4fd038))
- support multiple label groups per repository ([c8db1e7](https://github.com/tareqmy/gitwig/commit/c8db1e7))
- implement favorite star repos ([40b60a8](https://github.com/tareqmy/gitwig/commit/40b60a8))
- implement recently opened MRU stack group ([c553988](https://github.com/tareqmy/gitwig/commit/c553988))
- implement fuzzy jump-to-repo picker overlay ([faef755](https://github.com/tareqmy/gitwig/commit/faef755))
- implement label group collapsing and expanding ([4bef34b](https://github.com/tareqmy/gitwig/commit/4bef34b))
- show entire content of untracked files in diff ([51ada8d](https://github.com/tareqmy/gitwig/commit/51ada8d))

### Fixed
- sort case-insensitively by repository directory name instead of full path ([5a5c08a](https://github.com/tareqmy/gitwig/commit/5a5c08a))
- resync correct repository detail path in resync_detail ([d601235](https://github.com/tareqmy/gitwig/commit/d601235))
- add tags api fallback and improve version detection error message in install.sh ([fec08e8](https://github.com/tareqmy/gitwig/commit/fec08e8))

### Changed
- address clippy warnings ([37c857e](https://github.com/tareqmy/gitwig/commit/37c857e))

### Documentation
- document star, jump picker, and collapse shortcuts across help views ([5f6cd42](https://github.com/tareqmy/gitwig/commit/5f6cd42))

### Chore
- move install and uninstall scripts inside scripts/ directory ([e13b9f4](https://github.com/tareqmy/gitwig/commit/e13b9f4))

### Others
- refine group header heights and visibility in card/compact views ([88b3876](https://github.com/tareqmy/gitwig/commit/88b3876))

## [v2.3.1] - 2026-07-01
### Added
- improve compact view design and add column headers ([94fec5a](https://github.com/tareqmy/gitwig/commit/94fec5a))
- move indicators to legend and update for compatibility mode ([298937a](https://github.com/tareqmy/gitwig/commit/298937a))
- display legend vertically, add scrolling and symmetrical spacing ([e4833b5](https://github.com/tareqmy/gitwig/commit/e4833b5))
- change quit key to ctrl+q ([2640178](https://github.com/tareqmy/gitwig/commit/2640178))
- resolve v key conflict and increase legend width ([04d1733](https://github.com/tareqmy/gitwig/commit/04d1733))
- implement Signs & Symbols Legend popup ([e92877e](https://github.com/tareqmy/gitwig/commit/e92877e))
- implement Color-coded Divergence Badge ([1c4e9b1](https://github.com/tareqmy/gitwig/commit/1c4e9b1))
- implement Compact / Dense View Toggle ([65f03a3](https://github.com/tareqmy/gitwig/commit/65f03a3))
- display Last Activity relative timestamp on repository cards ([fcc9fbf](https://github.com/tareqmy/gitwig/commit/fcc9fbf))
- implement Repo Health / State Indicators on homepage cards ([a02f22f](https://github.com/tareqmy/gitwig/commit/a02f22f))

### Documentation
- update status bar shortcuts, roadmaps and instructions ([3d5c0e3](https://github.com/tareqmy/gitwig/commit/3d5c0e3))
- update ROADMAP.md visual enhancements status ([041aa18](https://github.com/tareqmy/gitwig/commit/041aa18))

## [v2.2.6] - 2026-07-01
### Added
- show implicit network actions and add logs for it ([9968ade](https://github.com/tareqmy/gitwig/commit/9968ade))
- add settings for user to change the terminal editor ([9a798bc](https://github.com/tareqmy/gitwig/commit/9a798bc))
- add shortcut for the open selected file in editor in files tab ([165eb46](https://github.com/tareqmy/gitwig/commit/165eb46))
- adding open selected file in terminal editor in files tab ([f902a21](https://github.com/tareqmy/gitwig/commit/f902a21))

### Fixed
- resolve Windows fzf picker failure by replacing find command with native directory walker ([32dc4a0](https://github.com/tareqmy/gitwig/commit/32dc4a0))

### Changed
- provide user option to trigger update check ([fb121ab](https://github.com/tareqmy/gitwig/commit/fb121ab))
- change update available display. instead of popup, show badge on top right ([206e6a2](https://github.com/tareqmy/gitwig/commit/206e6a2))

## [v2.2.5] - 2026-07-01
### Added
- **install**: add PowerShell installer and uninstaller for Windows support ([96258b6](https://github.com/tareqmy/gitwig/commit/96258b6))
- **branch**: implement remote branch deletion on the remote server (F03) ([49b33a8](https://github.com/tareqmy/gitwig/commit/49b33a8))

### Fixed
- prevent double event handling of navigation keys in branches tab ([88a4040](https://github.com/tareqmy/gitwig/commit/88a4040))
- **confirm**: align delete confirmation default to Cancel/No in Home view (F13/F17) ([a9b2f6c](https://github.com/tareqmy/gitwig/commit/a9b2f6c))
- **stash**: validate stash identity on confirmation to prevent index shifts (F11) ([80b39b5](https://github.com/tareqmy/gitwig/commit/80b39b5))
- **async**: associate background task messages with originating repositories (F10) ([388d7dd](https://github.com/tareqmy/gitwig/commit/388d7dd))
- **theme**: make theme load paths tolerant of file/dir write failures (F08) ([df85e5e](https://github.com/tareqmy/gitwig/commit/df85e5e))
- **logging**: implement log rotation and secure file permissions (S9) ([cebae67](https://github.com/tareqmy/gitwig/commit/cebae67))
- **security**: prevent path traversal in manually-built diff/patch hunks (S8) ([adaebbf](https://github.com/tareqmy/gitwig/commit/adaebbf))
- **security**: enforce directory permissions (S5) and char-boundary-safe slicing (S7) ([4f3b79f](https://github.com/tareqmy/gitwig/commit/4f3b79f))
- **commit**: unify commit index resolving via get_selected_commit() (F12) ([3b366d9](https://github.com/tareqmy/gitwig/commit/3b366d9))
- **commit**: reset commit selection on tab changes, repository load, and search events (F12) ([520ed37](https://github.com/tareqmy/gitwig/commit/520ed37))
- **security**: sanitize escape sequences and control characters from commit headers/messages/refs (S4) ([a503bdf](https://github.com/tareqmy/gitwig/commit/a503bdf))
- **security**: eliminate fzf shell command injection (S3) by using secure Command arg building ([3d72083](https://github.com/tareqmy/gitwig/commit/3d72083))
- **input**: map Enter key to cancel on destructive confirm popups (F04/F05) ([567a275](https://github.com/tareqmy/gitwig/commit/567a275))
- **security**: secure all spawned git commands (S2/S1) by setting protocol limits, validating ref names, and using -- separator ([79c308f](https://github.com/tareqmy/gitwig/commit/79c308f))
- **main**: add startup git dependency check (F01) and document prerequisites in README ([8289961](https://github.com/tareqmy/gitwig/commit/8289961))
- **config**: implement atomic config saves (R1a), corrupt config recovery (R1b), and unix config file/directory permissions (S5) ([3820e4b](https://github.com/tareqmy/gitwig/commit/3820e4b))
- **workspace**: make conflict panel abort/continue keys work by guarding generic workspace key handlers ([690d2bf](https://github.com/tareqmy/gitwig/commit/690d2bf))

### Changed
- update preview gif and cast ([ab3624a](https://github.com/tareqmy/gitwig/commit/ab3624a))
- roadmap update ([c1c150f](https://github.com/tareqmy/gitwig/commit/c1c150f))
- roadmap improvement ([b149ada](https://github.com/tareqmy/gitwig/commit/b149ada))

### Documentation
- document the SSH host key policy in README.md (S6) ([742fc82](https://github.com/tareqmy/gitwig/commit/742fc82))

### Others
- address remaining security and functional audit items ([ac254d6](https://github.com/tareqmy/gitwig/commit/ac254d6))

## [v2.2.4] - 2026-06-30
### Added
- support adding and deleting submodules inside the Submodules tab ([b8b27b1](https://github.com/tareqmy/gitwig/commit/b8b27b1))
- replace Overview tab with Submodules tab and add v Overview overlay ([06eb1d7](https://github.com/tareqmy/gitwig/commit/06eb1d7))

### Fixed
- resolve Node.js 20 deprecation and macOS runner migration warnings in workflows ([3f4d55a](https://github.com/tareqmy/gitwig/commit/3f4d55a))

### Documentation
- add audit remediation plan to .agent directory ([f1f2735](https://github.com/tareqmy/gitwig/commit/f1f2735))
- mark Git Submodules as complete in roadmap ([bc32c77](https://github.com/tareqmy/gitwig/commit/bc32c77))

## [v2.2.3] - 2026-06-30
### Added
- implement Git worktree support with interactive TUI management ([2e67501](https://github.com/tareqmy/gitwig/commit/2e67501))

### Fixed
- minor bugs solved for worktree flow ([8fe76fe](https://github.com/tareqmy/gitwig/commit/8fe76fe))

### Documentation
- update docs, instructions and shortcuts ([04fa601](https://github.com/tareqmy/gitwig/commit/04fa601))

### Others
- give meaningful text for each key bindings ([dae137e](https://github.com/tareqmy/gitwig/commit/dae137e))

## [v2.2.2] - 2026-06-29
### Added
- add keybindings category to settings panel with inline editing ([c17ba42](https://github.com/tareqmy/gitwig/commit/c17ba42))
- support keybindings configuration from external keybindings.toml ([1218db4](https://github.com/tareqmy/gitwig/commit/1218db4))
- add per-repository settings with global configuration fallbacks ([0debb56](https://github.com/tareqmy/gitwig/commit/0debb56))
- add startup self-update checking and in-app self-update triggering ([684f4a8](https://github.com/tareqmy/gitwig/commit/684f4a8))
- implement per-repository theme selection from Overview tab ([7f29d24](https://github.com/tareqmy/gitwig/commit/7f29d24))

## [v2.2.1] - 2026-06-29
### Added
- adding full commits/logs window from workspace ([9c4d774](https://github.com/tareqmy/gitwig/commit/9c4d774))
- added the gif from the asciinema cast file ([d03c776](https://github.com/tareqmy/gitwig/commit/d03c776))
- added uninstaller support ([2a81ec7](https://github.com/tareqmy/gitwig/commit/2a81ec7))
- added asciinema preview ([90c1252](https://github.com/tareqmy/gitwig/commit/90c1252))

### Fixed
- solution for installer in linux was facing error "gitwig: /lib/x86_64-linux-gnu/libc.so.6: version `GLIBC_2.39' not found (required by gitwig)" ([56797ef](https://github.com/tareqmy/gitwig/commit/56797ef))

### Documentation
- update docs and instructions ([a89765d](https://github.com/tareqmy/gitwig/commit/a89765d))

## [v2.1.7] - 2026-06-29
### Added
- **install**: add fzf auto-installation support to install.sh, bump version to 2.1.7 ([f7de82a](https://github.com/tareqmy/gitwig/commit/f7de82a))

## [v2.1.6] - 2026-06-29
### Added
- **cd**: add crates.io publish job to release workflow, bump version to 2.1.6 ([85f0a8b](https://github.com/tareqmy/gitwig/commit/85f0a8b))

## [v2.1.5] - 2026-06-29
### Fixed
- **cli**: add version and help CLI parsing to avoid terminal raw mode crash on script verification, bump version to 2.1.5 ([b14ca64](https://github.com/tareqmy/gitwig/commit/b14ca64))

## [v2.1.4] - 2026-06-29
### Fixed
- **cd**: replace taiki-e actions with native gh cli to support immutable releases, bump version to 2.1.4 ([a051292](https://github.com/tareqmy/gitwig/commit/a051292))

## [v2.1.3] - 2026-06-29
### Fixed
- **cd**: use draft releases to bypass immutability, bump version to 2.1.3 ([b4059e4](https://github.com/tareqmy/gitwig/commit/b4059e4))

## [v2.1.2] - 2026-06-29
### Fixed
- **install**: correct default branch name to master ([6be1ae6](https://github.com/tareqmy/gitwig/commit/6be1ae6))

### Others
- version 2.1.2 ([436c658](https://github.com/tareqmy/gitwig/commit/436c658))

## [v2.1.1] - 2026-06-29
### Added
- adding option to apply labels to repos ([d9a9603](https://github.com/tareqmy/gitwig/commit/d9a9603))
- implemented per file history view ([8b3be16](https://github.com/tareqmy/gitwig/commit/8b3be16))
- implement per-file history view and refine remote fetch behavior ([1b35c35](https://github.com/tareqmy/gitwig/commit/1b35c35))

### Changed
- allow mouse click on labels to filter out the repo list ([a8c5b4c](https://github.com/tareqmy/gitwig/commit/a8c5b4c))

### Documentation
- add curl-to-sh distribution installer and update roadmap ([6a47e19](https://github.com/tareqmy/gitwig/commit/6a47e19))

### Refactored
- **ui**: align codebase with style guide and theme rules ([90c38a3](https://github.com/tareqmy/gitwig/commit/90c38a3))

### Testing
- stop showing special symbols in files tab ([d631ec9](https://github.com/tareqmy/gitwig/commit/d631ec9))

### Others
- version 2.1.1 ([b2081e7](https://github.com/tareqmy/gitwig/commit/b2081e7))
- made a curl to sh distribution plan ([db3ddc0](https://github.com/tareqmy/gitwig/commit/db3ddc0))
- color applied based on type in files tab ([d55ac78](https://github.com/tareqmy/gitwig/commit/d55ac78))
- remotes tab will load from local on tab load, fetch on command from remote ([931f7b7](https://github.com/tareqmy/gitwig/commit/931f7b7))

## [v2.0.3] - 2026-06-28
### Added
- support j and k alongside up and down arrow navigation ([fea77fd](https://github.com/tareqmy/gitwig/commit/fea77fd))
- add interactive Stashing UI overlay ([e1448d6](https://github.com/tareqmy/gitwig/commit/e1448d6))

### Fixed
- tags tab bugs solved ([847e870](https://github.com/tareqmy/gitwig/commit/847e870))
- **publish**: specify version requirement for gitwig-core dependency ([9d23847](https://github.com/tareqmy/gitwig/commit/9d23847))

### Changed
- improve about gitwig window persentation ([4bb5861](https://github.com/tareqmy/gitwig/commit/4bb5861))

### Documentation
- docs and instructions updated ([574a223](https://github.com/tareqmy/gitwig/commit/574a223))
- updated docs ([0e4bf7f](https://github.com/tareqmy/gitwig/commit/0e4bf7f))

### Refactored
- **tabs**: clean up remote fetch triggers and align branches tab behavior ([fcfa6f2](https://github.com/tareqmy/gitwig/commit/fcfa6f2))

### Chore
- updated cargo publish structure ([e70305e](https://github.com/tareqmy/gitwig/commit/e70305e))

### Others
- reverse tags sort order ([957a4a6](https://github.com/tareqmy/gitwig/commit/957a4a6))
- increase stashing ui window size ([109982a](https://github.com/tareqmy/gitwig/commit/109982a))

## [v2.0.2] - 2026-06-28
### Added
- **workspace**: allow right arrow to go to inspect from diff panel ([9a962a7](https://github.com/tareqmy/gitwig/commit/9a962a7))
- enable inspect mouse click selection, update license inspirations, and organize status bar ([d81b229](https://github.com/tareqmy/gitwig/commit/d81b229))
- **inspect**: allow mouse clicks to select files in inspect view ([6719d37](https://github.com/tareqmy/gitwig/commit/6719d37))
- **commits**: use config.max_commits as load-more increment for G key ([f44cdb0](https://github.com/tareqmy/gitwig/commit/f44cdb0))
- **detail-help**: organize repository detail shortcuts into categorized sections ([877741b](https://github.com/tareqmy/gitwig/commit/877741b))
- **help**: organize shortcuts into categorized sections in help popup ([87121d8](https://github.com/tareqmy/gitwig/commit/87121d8))
- **logo**: add icon and implement ASCII logo in About popup ([6bf0259](https://github.com/tareqmy/gitwig/commit/6bf0259))
- **settings**: refine settings layout navigation and category clamping ([9e9bac7](https://github.com/tareqmy/gitwig/commit/9e9bac7))

### Fixed
- use fixed 1:1 ratio for staged and unstaged sections ([f7afdfc](https://github.com/tareqmy/gitwig/commit/f7afdfc))
- **inspect**: correct reversed panel focus transition directions for w and W keys ([78c5362](https://github.com/tareqmy/gitwig/commit/78c5362))
- **commit**: correct commit popup key bindings and unify maximize state ([9dd1b07](https://github.com/tareqmy/gitwig/commit/9dd1b07))
- **graph**: enable scrolling key bindings on Graph tab ([b38276f](https://github.com/tareqmy/gitwig/commit/b38276f))
- guarantee event queue is drained on all exit paths & restore missing panel shortcuts ([5d171a4](https://github.com/tareqmy/gitwig/commit/5d171a4))

### Performance
- clean up performance plan file ([7f76408](https://github.com/tareqmy/gitwig/commit/7f76408))
- implement Phase D of Large Repository Performance Plan ([ea1d502](https://github.com/tareqmy/gitwig/commit/ea1d502))

### Documentation
- **license**: add sourcetree, gitui, and lazygit to gitwig-core license ([edd24f4](https://github.com/tareqmy/gitwig/commit/edd24f4))
- bundle JetBrains Mono Nerd Font and add symbol support guide ([52d9517](https://github.com/tareqmy/gitwig/commit/52d9517))
- update the docs and instructions to match the current state of the app ([e276c3d](https://github.com/tareqmy/gitwig/commit/e276c3d))

### Refactored
- **app**: deconstruct monolithic app.rs into modules (Phase 4) ([f5237d1](https://github.com/tareqmy/gitwig/commit/f5237d1))
- deconstruct input.rs by delegating popup event handlers ([052b1e6](https://github.com/tareqmy/gitwig/commit/052b1e6))
- Modularize detail and popup keystroke routing (Phase 4b) ([56ca391](https://github.com/tareqmy/gitwig/commit/56ca391))
- Extract Confirm, Input popups, and mouse router (Phase 4b) ([30aac52](https://github.com/tareqmy/gitwig/commit/30aac52))
- Migrate UI components and CommitPopup to Event Bus architecture (Phase 4b) ([bca4f59](https://github.com/tareqmy/gitwig/commit/bca4f59))
- complete state migration for remaining UI components ([35140f3](https://github.com/tareqmy/gitwig/commit/35140f3))
- extract state into isolated ui components (Phase 4a) ([5e9828b](https://github.com/tareqmy/gitwig/commit/5e9828b))
- extract massive detail components from ui_detail.rs ([2b1bdf7](https://github.com/tareqmy/gitwig/commit/2b1bdf7))
- extract git popups from ui_detail.rs to src/popups ([4ef1910](https://github.com/tareqmy/gitwig/commit/4ef1910))
- extract main popups from ui.rs into src/popups ([60c3ba3](https://github.com/tareqmy/gitwig/commit/60c3ba3))
- extract command bar widget to components/cmd_bar.rs ([271524f](https://github.com/tareqmy/gitwig/commit/271524f))
- establish Phase 3 component architecture and event bus ([dea5c6e](https://github.com/tareqmy/gitwig/commit/dea5c6e))
- begin Phase 3 UI modularization (style and layout) ([7c863a9](https://github.com/tareqmy/gitwig/commit/7c863a9))
- extract git operations into gitwig-core workspace member ([ae939da](https://github.com/tareqmy/gitwig/commit/ae939da))
- update refactoring plan ([9451652](https://github.com/tareqmy/gitwig/commit/9451652))

### Testing
- change commit list footer to say showing latest commits ([8caa5ac](https://github.com/tareqmy/gitwig/commit/8caa5ac))

### Chore
- **ci**: complete phase 1 strict compile-time lint gates ([841f9ca](https://github.com/tareqmy/gitwig/commit/841f9ca))
- **ci**: complete phase 5 build & DX optimizations ([7ff9731](https://github.com/tareqmy/gitwig/commit/7ff9731))
- implement Phase 5 Build & DX optimizations ([65817fc](https://github.com/tareqmy/gitwig/commit/65817fc))
- Implement phase 1 of refactoring plan and fix tag push ([aad46ba](https://github.com/tareqmy/gitwig/commit/aad46ba))

### Others
- complete Phase 4 of refactoring by deconstructing remaining popup events ([6fc3674](https://github.com/tareqmy/gitwig/commit/6fc3674))
- finalize Phase 3 refactoring with scrollbar, syntax, and HomeTab deconstruction ([62897a1](https://github.com/tareqmy/gitwig/commit/62897a1))
- continue Phase 3 by decomposing monolithic tab event routing ([5321dbc](https://github.com/tareqmy/gitwig/commit/5321dbc))
- complete Phase 2 crate extraction by migrating remaining git2 queries ([14d7c0e](https://github.com/tareqmy/gitwig/commit/14d7c0e))
- take push all tags to remote confirmation ([a90e9af](https://github.com/tareqmy/gitwig/commit/a90e9af))

## [v1.3.2] - 2026-06-27
### Added
- cpu and memory tracker added in the bottom right status bar ([4d4db4d](https://github.com/tareqmy/gitwig/commit/4d4db4d))
- update cherrypick implementation ([46a3719](https://github.com/tareqmy/gitwig/commit/46a3719))
- implement yank commit hash option from commits panel in workspace ([13cf423](https://github.com/tareqmy/gitwig/commit/13cf423))

### Changed
- allow commit message popup resize with mouse ([62f2f41](https://github.com/tareqmy/gitwig/commit/62f2f41))

### Documentation
- update docs and instructions ([0b81e4b](https://github.com/tareqmy/gitwig/commit/0b81e4b))

### Others
- take abort merge confirmation from a popup ([bbec66c](https://github.com/tareqmy/gitwig/commit/bbec66c))
- resync after actions in branches tab ([b6e9d14](https://github.com/tareqmy/gitwig/commit/b6e9d14))
- lazyload was missing for files tab ([e63fee3](https://github.com/tareqmy/gitwig/commit/e63fee3))

## [v1.3.1] - 2026-06-26
### Added
- implement per-tab TTL caching with stale-while-revalidate (Phase C3) ([ca5dd0f](https://github.com/tareqmy/gitwig/commit/ca5dd0f))
- implement lazy ref map lookup with TTL caching (Phase C2) ([9f4204f](https://github.com/tareqmy/gitwig/commit/9f4204f))
- implement in-memory Detail view cache with TTL (Phase C1) ([3b553dc](https://github.com/tareqmy/gitwig/commit/3b553dc))
- implement streaming git graph log (Phase B3) ([d2cacd9](https://github.com/tareqmy/gitwig/commit/d2cacd9))
- implement tab-lazy loading and commit pagination (Phase B1 & B2) ([da55e21](https://github.com/tareqmy/gitwig/commit/da55e21))

### Performance
- implement Phase A of large repository performance plan ([1a96060](https://github.com/tareqmy/gitwig/commit/1a96060))

### Refactored
- optimize commit load performance with lazy file loading and optional signatures ([b800d46](https://github.com/tareqmy/gitwig/commit/b800d46))

## [v1.2.7] - 2026-06-26
### Added
- commit signatures implemented. display gpg/ssh commit verification status ([86da12c](https://github.com/tareqmy/gitwig/commit/86da12c))
- large repo performance plan added ([a4d1dcb](https://github.com/tareqmy/gitwig/commit/a4d1dcb))
- added about information ([1322254](https://github.com/tareqmy/gitwig/commit/1322254))

### Fixed
- implement cherrypick and revert commit ([18aa493](https://github.com/tareqmy/gitwig/commit/18aa493))
- trying to contain network actions like remote/tag fetch failure due to inaccessible or unauthorized ([605af54](https://github.com/tareqmy/gitwig/commit/605af54))

### Changed
- improved commit message popup editor ([fba76bd](https://github.com/tareqmy/gitwig/commit/fba76bd))
- settings to enable/disable of auto resync on tab change ([06ad8b7](https://github.com/tareqmy/gitwig/commit/06ad8b7))

### Refactored
- refactoring plan ([27519ac](https://github.com/tareqmy/gitwig/commit/27519ac))

### Others
- remote action progress bar in tags tab and remotes tab. ([9a4c0e8](https://github.com/tareqmy/gitwig/commit/9a4c0e8))

## [v1.2.6] - 2026-06-25
### Added
- introduce application update startup process for configs that need user input or confirmation ([5d8c433](https://github.com/tareqmy/gitwig/commit/5d8c433))
- add Compatibility Mode for robust rendering in restricted terminals ([2a79265](https://github.com/tareqmy/gitwig/commit/2a79265))
- add publish option ([3e0c67d](https://github.com/tareqmy/gitwig/commit/3e0c67d))

### Fixed
- trying to solve bug in windows where same command is applied multiple times ([f16b675](https://github.com/tareqmy/gitwig/commit/f16b675))

### Others
- show loading sign before a repo is loaded in the ui ([fabe74c](https://github.com/tareqmy/gitwig/commit/fabe74c))

## [v1.2.5] - 2026-06-25
### Added
- provide remote add/delete option in remotes tab ([9e9c378](https://github.com/tareqmy/gitwig/commit/9e9c378))
- roadmap added ([d2c65f1](https://github.com/tareqmy/gitwig/commit/d2c65f1))
- bulk add repo option ([05560ef](https://github.com/tareqmy/gitwig/commit/05560ef))
- add option git only in fzf search result ([ccb8011](https://github.com/tareqmy/gitwig/commit/ccb8011))
- add git clone/import option ([49328db](https://github.com/tareqmy/gitwig/commit/49328db))
- added helper makefile ([ec8e739](https://github.com/tareqmy/gitwig/commit/ec8e739))

### Fixed
- ensure errors and exceptions are logged ([bcffb54](https://github.com/tareqmy/gitwig/commit/bcffb54))
- added debug panel and added some logs ([882b94c](https://github.com/tareqmy/gitwig/commit/882b94c))

### Changed
- roadmap updated ([5023810](https://github.com/tareqmy/gitwig/commit/5023810))

### Removed
- when tag is deleted, delete it from the remote too with a warning ([8f6dde0](https://github.com/tareqmy/gitwig/commit/8f6dde0))

### Others
- message if external git app is not found ([6c52e3f](https://github.com/tareqmy/gitwig/commit/6c52e3f))
- incase fzf is enabled but fzf is not available fallback to text entry ([a469319](https://github.com/tareqmy/gitwig/commit/a469319))
- make usage of fzf optional ([07641bd](https://github.com/tareqmy/gitwig/commit/07641bd))

## [v1.2.4] - 2026-06-25
### Added
- roadmap added ([be7d2ee](https://github.com/tareqmy/gitwig/commit/be7d2ee))
- git remote add/delete option added in roadmap ([8c343c8](https://github.com/tareqmy/gitwig/commit/8c343c8))

### Fixed
- Fix settings popup fzf exclude multiline wrapping and cursor out of bounds ([814f9aa](https://github.com/tareqmy/gitwig/commit/814f9aa))

### Refactored
- extract wrap_excludes helper + unit test for fzf exclude multiline ([6379bd6](https://github.com/tareqmy/gitwig/commit/6379bd6))

### Others
- make default excluded folder in fzf as empty ([a0e46fd](https://github.com/tareqmy/gitwig/commit/a0e46fd))
- warning if fzf is not installed ([8b26916](https://github.com/tareqmy/gitwig/commit/8b26916))

## [v1.2.3] - 2026-06-24
### Added
- future features to be considered added ([e9ec48f](https://github.com/tareqmy/gitwig/commit/e9ec48f))

### Changed
- rename all reference from twig to gitwig ([403b3b2](https://github.com/tareqmy/gitwig/commit/403b3b2))
- give option to change commit message of amend commit ([fd3f2b7](https://github.com/tareqmy/gitwig/commit/fd3f2b7))

### Removed
- stage/unstage/discard all option in focused workspace ([c26ff50](https://github.com/tareqmy/gitwig/commit/c26ff50))

### Documentation
- mit license added ([7b93085](https://github.com/tareqmy/gitwig/commit/7b93085))

### Chore
- ci/cd workflow file for github actions ([9238507](https://github.com/tareqmy/gitwig/commit/9238507))

### Others
- since twig crate already exists changing to gitwig ([a440d91](https://github.com/tareqmy/gitwig/commit/a440d91))

## [v1.2.2] - 2026-06-24
### Added
- implement merge conflict resolution UI & integration flow ([273a2c0](https://github.com/tareqmy/gitwig/commit/273a2c0))
- did planning with thinking ai for merge conflict resolution support ([7cefcff](https://github.com/tareqmy/gitwig/commit/7cefcff))

### Changed
- progress bar ui improvement ([357db5a](https://github.com/tareqmy/gitwig/commit/357db5a))

### Others
- hunk by hunk conflict resolution ([c2c780d](https://github.com/tareqmy/gitwig/commit/c2c780d))
- right arrow to full content for files tab ([a0033c0](https://github.com/tareqmy/gitwig/commit/a0033c0))

## [v1.2.1] - 2026-06-24
### Added
- stashing option added ([dedd4eb](https://github.com/tareqmy/gitwig/commit/dedd4eb))
- implemente line by line diff allow or decline ([f70e11f](https://github.com/tareqmy/gitwig/commit/f70e11f))
- implement hunk stage/unstage/discard option in focused workspace ([14dd8ce](https://github.com/tareqmy/gitwig/commit/14dd8ce))

### Changed
- user option to provide preferred external git tui ([17b27ef](https://github.com/tareqmy/gitwig/commit/17b27ef))
- allow mouse selection of commits, files, branches etc from different lists ([31fb8d4](https://github.com/tareqmy/gitwig/commit/31fb8d4))
- update status bar based on panel on focus ([8caf30b](https://github.com/tareqmy/gitwig/commit/8caf30b))
- update all shortcuts in the status bar ([7ff8c1a](https://github.com/tareqmy/gitwig/commit/7ff8c1a))
- minor ui tweaks ([4e923c2](https://github.com/tareqmy/gitwig/commit/4e923c2))
- update refresh shortcut to R ([b610d95](https://github.com/tareqmy/gitwig/commit/b610d95))
- branch checkout improvement ([a5d17a2](https://github.com/tareqmy/gitwig/commit/a5d17a2))

### Removed
- provide stage all, unstage all and discard all option ([99cc9ce](https://github.com/tareqmy/gitwig/commit/99cc9ce))

### Documentation
- document updates ([9a54bfc](https://github.com/tareqmy/gitwig/commit/9a54bfc))
- updated docs and instructions ([a63d3c8](https://github.com/tareqmy/gitwig/commit/a63d3c8))

### Testing
- allow commit option from inspect ui ([bf7d1ce](https://github.com/tareqmy/gitwig/commit/bf7d1ce))
- implement full screen inspect from inspect diff section ([0941746](https://github.com/tareqmy/gitwig/commit/0941746))
- showing diff in hunks for inspect/focused workspace ui ([54424d5](https://github.com/tareqmy/gitwig/commit/54424d5))

### Others
- show progress bar for network actions ([144df4b](https://github.com/tareqmy/gitwig/commit/144df4b))
- provide search option on home page ([3c385f5](https://github.com/tareqmy/gitwig/commit/3c385f5))

## [v1.1.1] - 2026-06-23
### Added
- adding shortcut for lazygit - l ([610e0e1](https://github.com/tareqmy/gitwig/commit/610e0e1))
- configure FZF start directory, reverse cycle focus with Shift+W, and improve workspace panel navigation ([6c39905](https://github.com/tareqmy/gitwig/commit/6c39905))
- add option to limit fzf base directory from settings ([53964e6](https://github.com/tareqmy/gitwig/commit/53964e6))
- ensure same git repo is not added twice ([01b7ac6](https://github.com/tareqmy/gitwig/commit/01b7ac6))
- adding dropdown for theme selection in settings ([1b91142](https://github.com/tareqmy/gitwig/commit/1b91142))
- added a few builtin themes ([de1753a](https://github.com/tareqmy/gitwig/commit/de1753a))
- settings page added ([05418fc](https://github.com/tareqmy/gitwig/commit/05418fc))
- implement mouse dragging to resize panels in Stashes and Overview tabs ([613c269](https://github.com/tareqmy/gitwig/commit/613c269))
- implement mouse dragging to resize panels in Files and Branches tabs ([fe4c815](https://github.com/tareqmy/gitwig/commit/fe4c815))
- implement mouse dragging to resize panels in Workspace tab ([850ed1a](https://github.com/tareqmy/gitwig/commit/850ed1a))
- implement mouse dragging to resize panels in inspect view ([76e1650](https://github.com/tareqmy/gitwig/commit/76e1650))
- support full-screen inspection of uncommitted changes ([2d64065](https://github.com/tareqmy/gitwig/commit/2d64065))
- add full-screen commit inspect view in workspace tab ([d075b43](https://github.com/tareqmy/gitwig/commit/d075b43))
- suspend TUI and refresh repository state on return from interactive rebase ([06a15a2](https://github.com/tareqmy/gitwig/commit/06a15a2))
- add merge/rebase status bar shortcuts and implement branch rebase ([14b267c](https://github.com/tareqmy/gitwig/commit/14b267c))
- implement branch merge option in Branches tab ([0d852df](https://github.com/tareqmy/gitwig/commit/0d852df))
- add real-time commits search and filtering to Details tab ([6ab2279](https://github.com/tareqmy/gitwig/commit/6ab2279))
- fetch all refs on Remotes tab and update progress overlays ([2f05423](https://github.com/tareqmy/gitwig/commit/2f05423))
- add multi-remote picker for push/fetch operations ([3da8dd7](https://github.com/tareqmy/gitwig/commit/3da8dd7))
- adding stats in the overview tab ([06663d4](https://github.com/tareqmy/gitwig/commit/06663d4))
- configure themes and FZF search in config.toml, enhance commit popup scroll ([d307da2](https://github.com/tareqmy/gitwig/commit/d307da2))
- support for mouse scroll in the compose commit message ([0836db9](https://github.com/tareqmy/gitwig/commit/0836db9))
- implement repository pinning and prevent duplicate additions ([3691208](https://github.com/tareqmy/gitwig/commit/3691208))
- integrate interactive fzf directory picker for adding repos ([cc4b409](https://github.com/tareqmy/gitwig/commit/cc4b409))
- add main page sorting and fix commit amend functionality ([9035c88](https://github.com/tareqmy/gitwig/commit/9035c88))
- support amend last commit ([d8fe837](https://github.com/tareqmy/gitwig/commit/d8fe837))
- apply stash option added ([67ce919](https://github.com/tareqmy/gitwig/commit/67ce919))
- adding stashes tab ([e6a44e8](https://github.com/tareqmy/gitwig/commit/e6a44e8))
- tag deletion option added ([d843df7](https://github.com/tareqmy/gitwig/commit/d843df7))
- added remotes tab ([170047b](https://github.com/tareqmy/gitwig/commit/170047b))
- added tags tab ([0246858](https://github.com/tareqmy/gitwig/commit/0246858))
- option to tag a commit is added ([9ac0c12](https://github.com/tareqmy/gitwig/commit/9ac0c12))
- shorten file status labels and add remote branch pull capability ([7dce8c3](https://github.com/tareqmy/gitwig/commit/7dce8c3))
- add branch details in branches tab ([42dc47c](https://github.com/tareqmy/gitwig/commit/42dc47c))
- adding push branch to remote option ([d0acc2f](https://github.com/tareqmy/gitwig/commit/d0acc2f))
- branch create and delete option added ([f8fa256](https://github.com/tareqmy/gitwig/commit/f8fa256))
- add files tab. showing the files in the repo ([cd1531b](https://github.com/tareqmy/gitwig/commit/cd1531b))
- adding option g to deploy gitui for a repo ([b812a3e](https://github.com/tareqmy/gitwig/commit/b812a3e))
- add branches tab, background fetch, and checkout features ([f9a44c7](https://github.com/tareqmy/gitwig/commit/f9a44c7))
- add branches view ([dc27ed9](https://github.com/tareqmy/gitwig/commit/dc27ed9))

### Fixed
- bug with focus shift is solved ([c217dd5](https://github.com/tareqmy/gitwig/commit/c217dd5))
- allow dirty file revert option ([b21751c](https://github.com/tareqmy/gitwig/commit/b21751c))
- prevent existing repo from getting added ([5e4e273](https://github.com/tareqmy/gitwig/commit/5e4e273))

### Changed
- branches, tags, graph improvement ([c5934e0](https://github.com/tareqmy/gitwig/commit/c5934e0))
- update the config.toml ([5186a22](https://github.com/tareqmy/gitwig/commit/5186a22))
- provide user with resync option to update the files with current state ([d4d9010](https://github.com/tareqmy/gitwig/commit/d4d9010))
- improvement on scroll, page size is made user customizable through settings, home and end buttons provided ([675a985](https://github.com/tareqmy/gitwig/commit/675a985))
- improved search in git logs ([9b115a2](https://github.com/tareqmy/gitwig/commit/9b115a2))
- improvement on files tab layout ([836266a](https://github.com/tareqmy/gitwig/commit/836266a))
- improvement on focus shift in workspace ([892725f](https://github.com/tareqmy/gitwig/commit/892725f))
- updated the main page styling ([76561dd](https://github.com/tareqmy/gitwig/commit/76561dd))
- rename details tab to workspace tab ([ea8dad4](https://github.com/tareqmy/gitwig/commit/ea8dad4))
- minor appearance improvement ([c1c0df5](https://github.com/tareqmy/gitwig/commit/c1c0df5))
- improved tab header presentation ([44941a1](https://github.com/tareqmy/gitwig/commit/44941a1))
- improved stashes tab ([9f527a3](https://github.com/tareqmy/gitwig/commit/9f527a3))
- moved the overview popup to overview tab ([3319507](https://github.com/tareqmy/gitwig/commit/3319507))
- updated all the status bars ([48faba4](https://github.com/tareqmy/gitwig/commit/48faba4))
- simplified status mapping for changed files to single letter ([9df12dc](https://github.com/tareqmy/gitwig/commit/9df12dc))
- use tab to change tabs. use w to cycle focus in details tab ([1b74460](https://github.com/tareqmy/gitwig/commit/1b74460))
- allow mouse scroll ([57ec9b4](https://github.com/tareqmy/gitwig/commit/57ec9b4))
- mouse click to change tabs ([3afeefe](https://github.com/tareqmy/gitwig/commit/3afeefe))
- giving users option to open a repo in gitui ([e4927bb](https://github.com/tareqmy/gitwig/commit/e4927bb))

### Removed
- option to delete a stash ([60830e6](https://github.com/tareqmy/gitwig/commit/60830e6))

### Documentation
- update docs and instructions ([113a14c](https://github.com/tareqmy/gitwig/commit/113a14c))
- doc update ([9c747c2](https://github.com/tareqmy/gitwig/commit/9c747c2))
- document resizable split panels feature and add help shortcuts ([090a626](https://github.com/tareqmy/gitwig/commit/090a626))
- docs and instructions updated ([fadafc8](https://github.com/tareqmy/gitwig/commit/fadafc8))
- update the docs ([8081e42](https://github.com/tareqmy/gitwig/commit/8081e42))
- updated all the docs with current state of the application ([2cf5bf5](https://github.com/tareqmy/gitwig/commit/2cf5bf5))

### Testing
- go to inspect ui if enter is pressed from search ui ([24901b9](https://github.com/tareqmy/gitwig/commit/24901b9))
- allow enter to go to inspect from commits window also ([55c782c](https://github.com/tareqmy/gitwig/commit/55c782c))
- right arrow to go to inspect page from committed files or uncommitted files ([4aaaf7f](https://github.com/tareqmy/gitwig/commit/4aaaf7f))
- inspect for commit show commit info on left panel top section ([ae74920](https://github.com/tareqmy/gitwig/commit/ae74920))
- move theme into themes folder and an option to select a specific theme. the name of the file should be same as mentioned in the config.toml ([924df8f](https://github.com/tareqmy/gitwig/commit/924df8f))

### Chore
- upgrade version to v1.1.1 ([1cfb344](https://github.com/tareqmy/gitwig/commit/1cfb344))

### Others
- fzf exlude folder edit option from settings ([af51a9a](https://github.com/tareqmy/gitwig/commit/af51a9a))
- scroll option in settings popup ([5462641](https://github.com/tareqmy/gitwig/commit/5462641))
- scroll only the marked entries after search is done ([263269f](https://github.com/tareqmy/gitwig/commit/263269f))
- settings to load number of commits applied ([3217df0](https://github.com/tareqmy/gitwig/commit/3217df0))
- provided option to find a file in files tab ([16b6212](https://github.com/tareqmy/gitwig/commit/16b6212))
- dont want the fullpath on the main page ([31d55ba](https://github.com/tareqmy/gitwig/commit/31d55ba))
- allow scrolling content of the file from files tab in content section ([8cef75b](https://github.com/tareqmy/gitwig/commit/8cef75b))
- remove header for inspect uncommitted changes ([2cd8986](https://github.com/tareqmy/gitwig/commit/2cd8986))
- in case of window width not enough for all the tab headers, shorten it to single character ([f19f1f6](https://github.com/tareqmy/gitwig/commit/f19f1f6))
- allow reversing the current sorting ([6c939ba](https://github.com/tareqmy/gitwig/commit/6c939ba))
- show progress bar for network operations ([af076c0](https://github.com/tareqmy/gitwig/commit/af076c0))
- allow esc to exit application from home page ([9cd186a](https://github.com/tareqmy/gitwig/commit/9cd186a))
- show content of files and folders in the files tab ([dfac276](https://github.com/tareqmy/gitwig/commit/dfac276))
- option to push tag and push all tags from tags tab ([ba75faf](https://github.com/tareqmy/gitwig/commit/ba75faf))
- rearrange the tabs ([6718e39](https://github.com/tareqmy/gitwig/commit/6718e39))
- made help popup scrollable ([7873614](https://github.com/tareqmy/gitwig/commit/7873614))
- show the diff of the first file off the selected line in the commits section ([49cc4ef](https://github.com/tareqmy/gitwig/commit/49cc4ef))
- files and directory in interactive tree mode ([053c5bb](https://github.com/tareqmy/gitwig/commit/053c5bb))

## [v1.0.1] - 2026-06-19
### Added
- implement history graph view and update tab selection keys ([9f577a2](https://github.com/tareqmy/gitwig/commit/9f577a2))
- control + c to finish compose and commit enter to insert new line ([fab05bf](https://github.com/tareqmy/gitwig/commit/fab05bf))
- add commit popup window and execute Git commits ([fcdb969](https://github.com/tareqmy/gitwig/commit/fcdb969))
- scrollable diff panel and staging area file selection ([9dd5d19](https://github.com/tareqmy/gitwig/commit/9dd5d19))
- **ui**: add committed file navigation and diff viewer to detail view ([2809f61](https://github.com/tareqmy/gitwig/commit/2809f61))
- **detail**: ref badges, detail help overlay, and commit page navigation ([60c3229](https://github.com/tareqmy/gitwig/commit/60c3229))
-     feat(detail): commit row selection with keyboard navigation ([04c04bd](https://github.com/tareqmy/gitwig/commit/04c04bd))
-     ui: add detail view panel focus cycling via Tab ([ac04317](https://github.com/tareqmy/gitwig/commit/ac04317))
- **ui**: split detail bottom into staging panels; add overview popup ([da026a9](https://github.com/tareqmy/gitwig/commit/da026a9))
- add PgDn / PgUp navigation in the main list ([97b6a61](https://github.com/tareqmy/gitwig/commit/97b6a61))
- add Enter-to-open detail view with git2-backed repo info ([a6aa24d](https://github.com/tareqmy/gitwig/commit/a6aa24d))
- add per-item filesystem status indicator (git / dir / missing) ([f668963](https://github.com/tareqmy/gitwig/commit/f668963))
- add ? shortcut showing all keybindings in a help overlay ([4fcca2d](https://github.com/tareqmy/gitwig/commit/4fcca2d))
- add in-app add/edit/delete for config items ([cafb170](https://github.com/tareqmy/gitwig/commit/cafb170))
- adding helpful comments ([90f2f3a](https://github.com/tareqmy/gitwig/commit/90f2f3a))
- meta data added ([3832957](https://github.com/tareqmy/gitwig/commit/3832957))
- adding browsing between elements with arrow / j,k ([cd285aa](https://github.com/tareqmy/gitwig/commit/cd285aa))

### Fixed
- Show commits table at the top of detail view and revert main screen changes ([3c3b751](https://github.com/tareqmy/gitwig/commit/3c3b751))
- fix the status line at the bottom of the window ([0b23774](https://github.com/tareqmy/gitwig/commit/0b23774))
- config corrected ([5fa48c8](https://github.com/tareqmy/gitwig/commit/5fa48c8))

### Changed
- minor style updates ([48433e3](https://github.com/tareqmy/gitwig/commit/48433e3))
- allow mouse click to select and double click to enter detail view from the home page ([0aef73f](https://github.com/tareqmy/gitwig/commit/0aef73f))
- Update the shortcuts window and bottom bar ([082163d](https://github.com/tareqmy/gitwig/commit/082163d))
- allow mouse click to change focus on a panel ([d966a84](https://github.com/tareqmy/gitwig/commit/d966a84))
- show changed files when commit is selected. for uncommitted row show staging area ([da3ed1b](https://github.com/tareqmy/gitwig/commit/da3ed1b))
- use ~/.twig/ as the canonical config directory ([9ad35ad](https://github.com/tareqmy/gitwig/commit/9ad35ad))
- polish UI for professional look across light and dark terminals ([3dd60b3](https://github.com/tareqmy/gitwig/commit/3dd60b3))

### Removed
-   • Overview popup ( build_repo_body ): Removed the Working Tree section — the popup now shows only Overview, Repository, and Sync.   • Staging Area panel ( draw_staging_panels ): The left "Staging Area" block is now split vertically into two sub-panels via  draw_file_subpanel :       • Top — "Staged" (green title) — lists all staged files with their labels, or shows "Nothing staged" centred if empty       • Bottom — "Unstaged" (yellow title) — lists all unstaged files, or shows "No unstaged changes" if empty   • Removed dead code:  push_subsection_header  and  append_working_tree  are gone (were only used by the removed Working Tree section). ([a585a34](https://github.com/tareqmy/gitwig/commit/a585a34))
- removed scrollbar. didnt like it. it was overlapping my right border of items ([cd20232](https://github.com/tareqmy/gitwig/commit/cd20232))

### Documentation
- docs updated ([8317b76](https://github.com/tareqmy/gitwig/commit/8317b76))
- docs updated to current app state ([895f28c](https://github.com/tareqmy/gitwig/commit/895f28c))
- ai agent instructions added ([2adfd74](https://github.com/tareqmy/gitwig/commit/2adfd74))
- added initial readme ([3aadd9e](https://github.com/tareqmy/gitwig/commit/3aadd9e))

### Refactored
- improve status bar layout, commit behavior, and key labels ([ac7c162](https://github.com/tareqmy/gitwig/commit/ac7c162))

### Testing
- Commit details improved   1. Focus State for Commit Details: Added highlighting for the  Commit Details  panel border when it is selected/focused.   2. Keyboard Scrolling: Enabled vertical scrolling via  ↑ / k  and  ↓ / j  when  CommitDetails  has keyboard focus.   3. Commit Messages formatting: Modified  CommitEntry  to store and display multi-line commit messages so they scroll appropriately.   4. Hit-Testing/Mouse clicks: Added hit-testing for mouse clicks on the  Commit Details  panel region, allowing users to focus it by clicking on it.   5. Code Quality: Formatted code to adhere to  cargo fmt  constraints. The application builds and passes all check gates ( cargo check ,  cargo test ,  cargo   clippy ) successfully. ([d6f14ac](https://github.com/tareqmy/gitwig/commit/d6f14ac))

### Chore
- upgrade to version v1.0.1 ([328a915](https://github.com/tareqmy/gitwig/commit/328a915))

### Others
- enter to confirm commit ([cd2b31a](https://github.com/tareqmy/gitwig/commit/cd2b31a))
- Organized the status bar ([677faa5](https://github.com/tareqmy/gitwig/commit/677faa5))
- show diff of staged/unstaged files in the staging details panel ([1088038](https://github.com/tareqmy/gitwig/commit/1088038))
- scrollable diff panel ([3316c5e](https://github.com/tareqmy/gitwig/commit/3316c5e))
- show both local and remote branch in the commits ([9c26d4a](https://github.com/tareqmy/gitwig/commit/9c26d4a))
- polish detail view header and panel borders ([258cd2f](https://github.com/tareqmy/gitwig/commit/258cd2f))
- Split main view horizontally to show recent commits table on top ([3dcbd71](https://github.com/tareqmy/gitwig/commit/3dcbd71))
- Display staged and unstaged files in repository details view ([8904431](https://github.com/tareqmy/gitwig/commit/8904431))
- show current branch name on each overview card ([a24bf25](https://github.com/tareqmy/gitwig/commit/a24bf25))
- show empty-state message when no items are tracked ([2fbc13e](https://github.com/tareqmy/gitwig/commit/2fbc13e))
- make event-loop poll interval configurable via config.toml ([9c06761](https://github.com/tareqmy/gitwig/commit/9c06761))
- show staged/modified/untracked/ahead counts in per-card indicator ([864a08a](https://github.com/tareqmy/gitwig/commit/864a08a))
- split main.rs into app, ui, and input modules ([a6c728c](https://github.com/tareqmy/gitwig/commit/a6c728c))
- migration from tui-rs to ratatui ([77ccab2](https://github.com/tareqmy/gitwig/commit/77ccab2))
- the config is looked according to preference of 1. command 2. local config directory 3. ~/.config/twig ([e0397cc](https://github.com/tareqmy/gitwig/commit/e0397cc))
- make it scrollable with a constant help/status line at the bottom ([ca9b0da](https://github.com/tareqmy/gitwig/commit/ca9b0da))
- make it scrollable! NOTE: the last item is not shown! ([95ffd47](https://github.com/tareqmy/gitwig/commit/95ffd47))
- using a config.toml to load repo list and show boxes for each repo ([beb9fd0](https://github.com/tareqmy/gitwig/commit/beb9fd0))
- initial commit with a hello message with a border ([794bb44](https://github.com/tareqmy/gitwig/commit/794bb44))
