# Gitwig - Panel & UI Window Directory

This document lists and describes all the user interface panels, tabs, views, overlay pickers, inputs, and modal dialogs implemented in Gitwig, along with their available keyboard shortcuts.

---

## 1. Home Screen (Repository List View)
The primary dashboard shown when Gitwig starts up.

*   **Repository List Panel**: Displays cards for all configured/scanned repositories. Supports Standard and Compact layouts.
    *   *Shortcuts*:
        *   `↑` / `k` / `up` : Move selection up
        *   `↓` / `j` / `down` : Move selection down
        *   `PgUp` / `pageup` : Jump one page up
        *   `PgDn` / `pagedown` : Jump one page down
        *   `Home` / `home` : Go to top of list
        *   `End` / `end` : Go to bottom of list
        *   `Enter` / `→` / `right` : Open repository details
        *   `Space` : Toggle selection for batch actions
        *   `p` : Toggle pin status of selected item
        *   `*` : Toggle Starred / Favorite status of selected item
        *   `y` : Yank absolute path to clipboard
        *   `/` : Open fuzzy Jump-to-Repo picker
        *   `f` : Enter repository list search query input mode
        *   `ctrl-f` : Open global code search across all repositories
        *   `R` : Refresh status of selected item manually
        *   `F` : Bulk fetch all repositories concurrently
        *   `o` : Cycle sorting criteria (Name, Path, Last Commit, Status)
        *   `O` : Toggle reverse sorting
        *   `v` : Toggle compact single-row list layout
        *   `a` : Add a repository path manually
        *   `A` : Bulk add subdirectories of a directory
        *   `i` : Import / clone a remote repository
        *   `e` : Edit selected repository entry
        *   `D` : Delete selected repository entry
        *   `l` : Edit custom labels of selected item
        *   `t` : Open terminal shell in repository path
        *   `g` : Launch preferred external Git GUI client
        *   `d` : Open debug logs panel
        *   `V` : Open about dialog
        *   `h` : Show signs & symbols legend popup
        *   `u` : Check for updates manually
        *   `s` : Open settings view
        *   `?` : Toggle help overlay
        *   `ctrl-q` : Quit Gitwig
*   **Grouping Sidebar (Left)**: Visible when `Show Grouping` is enabled. Groups repositories into Recent, Starred, and custom Labels.
    *   *Shortcuts*:
        *   `←` / `→` : Toggle focus between sidebar groups and repository list
        *   `Space` / `Enter` : Collapse / expand label groups (when focused on header)
*   **Bottom Command/Status Bar**: Reflects current keyboard context, active mode, background fetches, and CPU/memory statistics.

---

## 2. Repository Detail / Workspace View
Triggered by opening a repository.

### Sidebar Panel (Left)
*   **Tab Selector Panel**: Navigates between primary and advanced views.
    *   *Shortcuts*:
        *   `Esc` / `q` / `Q` : Close details / Return to Home Screen
        *   `?` : Toggle detail view shortcut help overlay
        *   `w` : Cycle focus forward through panels
        *   `W` : Cycle focus backward through panels
        *   `Tab` : Cycle tab forward
        *   `Shift+Tab` / `backtab` : Cycle tab backward
        *   `R` : Resync active tab details manually
        *   `1` - `7` : Jump directly to Tabs 1-7 (or Tabs 8-11 in advanced group)
        *   `Z` : Toggle Advanced Tab group (Worktrees, Submodules, Reflog, Forge)
        *   `O` : Show repository Overview overlay

### Main Workspace Tabs

#### Tab 0: Workspace (Commits History & Workspace Staging)
*   *Commits List Panel (Left)*: Lists Git commit logs with authors, dates, and messages. Includes an "Uncommitted changes" virtual item at the very top.
    *   *Shortcuts*:
        *   `↑` / `k` / `K` : Move selection up
        *   `↓` / `j` / `J` : Move selection down
        *   `PgUp` / `PgDn` : Scroll commit list
        *   `Home` / `End` : Jump to top / bottom of list
        *   `Enter` / `→` : Inspect selected commit (enters Commit Inspection Layout)
        *   `t` / `T` : Create tag at selected commit
        *   `b` / `B` : Create new branch at selected commit
        *   `y` / `Y` : Yank selected commit hash
        *   `v` / `V` : Revert selected commit
        *   `p` / `P` : Cherry-pick selected commit
        *   `i` / `I` : Run interactive rebase from selected commit
        *   `s` / `S` : Open stashing UI panel
        *   `c` : Open Commit message dialog
        *   `C` : Open Commit message dialog with Amend
        *   `/` : Fuzzy search commits list
        *   `f` : Open search column picker popup
        *   `l` / `L` : Open full-screen Logs view
        *   `G` : Load more commits
*   *Commit Info Panel (Top Right)*: Metadata and descriptions of the selected commit.
*   *Files List Panel (Bottom Right)*: List of changed files in the selected commit. Displays inline diff on selection.

#### Tab 1: Files Tab (Working Directory Browser)
*   *File Tree Panel (Left)*: Lists all files in the current workspace directory.
    *   *Shortcuts*:
        *   `↑` / `k` / `K` : Move selection up
        *   `↓` / `j` / `J` : Move selection down
        *   `PgUp` / `PgDn` : Scroll file list
        *   `Home` / `End` : Jump to top / bottom of file tree
        *   `>` / `.` : Expand folder in tree
        *   `<` / `,` : Collapse folder in tree
        *   `b` / `B` : Toggle git blame panel
        *   `n` / `N` : Toggle line numbers in content viewer
        *   `e` / `o` : Open selected file in default terminal editor
        *   `H` : View commit history of the selected file
        *   `/` : Launch fuzzy file search picker
*   *File Content Viewer (Right)*: Displays code content of the selected file. Supports git blame when toggled.
    *   *Shortcuts when focused*:
        *   `↑` / `k` / `K` : Scroll content up
        *   `↓` / `j` / `J` : Scroll content down
        *   `PgUp` / `PgDown` : Scroll content page up / down
        *   `Home` / `End` : Scroll to top / bottom of content
        *   `→` : Enter full-screen view mode
        *   `←` : Exit full-screen view mode

#### Tab 2: Graph Tab
*   *Commit Graph Panel*: A full-width panel displaying a visual branch-merging history graph.
    *   *Shortcuts*:
        *   `↑` / `k` / `K` : Move selection up
        *   `↓` / `j` / `J` : Move selection down
        *   `PgUp` / `PgDn` : Scroll graph page
        *   `Home` / `End` : Jump to top / bottom
        *   `Enter` / `→` : Inspect selected commit (enters Commit Inspection Layout)
        *   `y` / `Y` : Yank selected commit hash

#### Tab 3: Branches Tab
*   *Branches Lists (Local left, Remote right)*: Displays repository branches.
    *   *Shortcuts*:
        *   `←` / `→` : Switch focus between Local and Remote branches lists
        *   `↑` / `k` / `K` : Move selection up
        *   `↓` / `j` / `J` : Move selection down
        *   `PgUp` / `PgDn` / `Home` / `End` : Navigate branch lists
        *   `Enter` : Checkout selected branch
        *   `c` / `C` : Create new branch
        *   `D` : Delete selected branch
        *   `m` / `M` : Merge selected branch into current branch
        *   `r` : Rebase current branch onto selected branch
        *   `i` / `I` : Interactive rebase of current branch onto selected branch
        *   `p` : Pull remote changes (for local branches)
        *   `P` : Push selected branch to remote (for local branches)
        *   `/` : Fuzzy search branches

#### Tab 4: Tags Tab
*   *Tags List Panel*: Lists local and remote tags.
    *   *Shortcuts*:
        *   `↑` / `k` / `K` / `↓` / `j` / `J` : Move selection
        *   `PgUp` / `PgDn` / `Home` / `End` : Navigate tag list
        *   `Enter` : Checkout selected tag
        *   `D` : Delete selected tag
        *   `p` : Push selected tag to remote
        *   `P` : Push all tags to remote
        *   `f` / `F` : Fetch remote tags
        *   `/` : Fuzzy search tags

#### Tab 5: Remotes Tab
*   *Remotes List Panel*: Lists configured remote sources.
    *   *Shortcuts*:
        *   `↑` / `k` / `K` / `↓` / `j` / `J` : Move selection
        *   `PgUp` / `PgDn` / `Home` / `End` : Navigate remote list
        *   `a` / `A` : Add a new remote
        *   `D` : Delete selected remote
        *   `f` / `F` : Fetch selected remote (opens picker if multiple remotes exist)

#### Tab 6: Stashes Tab
*   *Stashes Panel (Left)*: Lists stashed changes.
    *   *Shortcuts*:
        *   `↑` / `k` / `K` / `↓` / `j` / `J` : Move selection
        *   `PgUp` / `PgDn` / `Home` / `End` : Navigate stashes list
        *   `a` / `A` : Apply selected stash
        *   `s` / `S` : Create a new stash (opens stash message input)
        *   `D` : Delete selected stash
*   *Stashed Files Panel (Top Right)*: Lists files in selected stash.
*   *Stash Diff Panel (Bottom Right)*: Diff preview of selected stashed files.

#### Tab 7: Worktrees Tab (Advanced Tab Group)
*   *Worktrees List Panel*: Lists worktrees.
    *   *Shortcuts*:
        *   `↑` / `k` / `K` / `↓` / `j` / `J` : Move selection
        *   `PgUp` / `PgDn` / `Home` / `End` : Navigate worktrees list
        *   `Enter` : Open selected worktree in new Gitwig context (adds to list and focuses it)
        *   `a` : Add new worktree
        *   `D` : Remove selected worktree
        *   `l` : Toggle lock status of selected worktree (asks for reason if locking)
        *   `p` : Prune stale worktree metadata

#### Tab 8: Submodules Tab (Advanced Tab Group)
*   *Submodules List Panel*: Lists submodules.
    *   *Shortcuts*:
        *   `↑` / `k` / `K` / `↓` / `j` / `J` : Move selection
        *   `a` : Add new submodule
        *   `D` : Delete selected submodule

#### Tab 9: Reflog Tab (Advanced Tab Group)
*   *Reflog List Panel*: Lists local reference logs.
    *   *Shortcuts*:
        *   `↑` / `k` / `K` / `↓` / `j` / `J` : Move selection
        *   `PgUp` / `PgDn` / `Home` / `End` : Navigate reflog list
        *   `Enter` / `Space` : Checkout commit OID of selected reflog entry

#### Tab 10: Forge Tab (Advanced Tab Group)
*   *Forge Issues Panel*: Displays PRs, issues, and CI/CD status.
    *   *Shortcuts*:
        *   `↑` / `k` / `K` / `↓` / `j` / `J` : Move selection
        *   `PgUp` / `PgDn` / `Home` / `End` : Navigate issues list
        *   `Enter` : Checkout branch corresponding to selected issue
        *   `o` : Open selected issue in web browser

---

## 3. Commit Inspection Layout (`Mode::Inspect`)
Workspace layout for inspecting commits and diffs in details.

*   *General Shortcuts*:
    *   `Esc` / `q` / `Q` : Exit inspection, return to Workspace Tab
    *   `Tab` / `w` : Cycle focus forward between Staged, Unstaged, Conflicts, and Diff viewer panels
    *   `Shift+Tab` / `W` : Cycle focus backward between panels
    *   `?` : Toggle inspect view shortcut help
*   *Staging Lists Shortcuts (Staged/Unstaged/Conflicts)*:
    *   `↑` / `k` / `K` / `↓` / `j` / `J` : Navigate file lists
    *   `Enter` : Stage selected file (if in Unstaged list) or Unstage selected file (if in Staged list)
    *   `a` : Stage all changes (if in Unstaged) or Unstage all changes (if in Staged)
    *   `x` : Discard changes in selected unstaged file (opens confirmation dialog)
    *   `X` : Discard all unstaged changes in repository (opens confirmation dialog)
    *   `c` : Start commit message dialog
    *   `C` : Start commit message dialog with Amend
    *   `s` / `S` : Open stashing UI panel
*   *Diff Viewer Panel Shortcuts*:
    *   `↑` / `k` / `K` / `↓` / `j` / `J` : Navigate lines / hunks (or scroll content)
    *   `PgUp` / `PgDn` / `Home` / `End` : Scroll diff viewer
    *   `→` : Expand diff viewer to full screen (sets `inspect_full_diff = true`)
    *   `←` : Exit full-screen diff viewer
    *   `l` / `L` : Toggle line-by-line stage/discard mode (switches between Hunk mode and Line mode)
    *   `Enter` / `s` : Stage selected line or hunk (when focused on Unstaged file diff)
    *   `Enter` / `u` : Unstage selected line or hunk (when focused on Staged file diff)
    *   `x` / `Delete` : Discard selected line or hunk (when focused on Unstaged file diff)

---

## 4. Conflict Resolution Panel (Inside Workspace Merge Conflict State)
Triggers when staging changes encounters merge conflicts.

*   *Shortcuts (in Staging lists or Diff viewer)*:
    *   `o` : Accept OURS version of conflict
    *   `t` : Accept THEIRS version of conflict
    *   `r` : Mark conflict as resolved
    *   `A` : Abort merge (opens confirmation dialog)
    *   `C` : Continue merge (opens confirmation dialog)

---

## 5. Standalone Views (Full Screen / Dedicated Panels)

### Logs View (`Mode::Logs`)
Full-screen git commits history.
*   *Shortcuts*:
    *   `Esc` / `q` / `Q` : Return to Workspace Tab
    *   `↑` / `k` / `K` / `↓` / `j` / `J` : Navigate commits
    *   `PgUp` / `PgDn` / `Home` / `End` : Scroll commits list
    *   `Enter` : Inspect selected commit (enters Commit Inspection Layout)
    *   `/` : Start commit fuzzy search picker
    *   `f` : Open search column picker popup
    *   `G` : Load more commits

### Per-File History View (`Mode::FileHistory`)
Commits history of selected file.
*   *Shortcuts*:
    *   `Esc` / `q` / `Q` : Return to detail view
    *   `Tab` / `w` / `W` / `←` / `→` : Toggle keyboard focus between Revisions list and Diff panel
    *   `↑` / `k` / `K` / `↓` / `j` / `J` : Scroll list (when Revisions is focused) or scroll diff (when Diff is focused)
    *   `PgUp` / `PgDn` / `Home` / `End` : Scroll list / diff

### Debug Logs Panel (`Mode::DebugLogs`)
App logs viewer.
*   *Shortcuts*:
    *   `Esc` / `q` / `D` / `l` / `L` : Return to Home Screen
    *   `↑` / `k` / `↓` / `j` : Scroll logs line-by-line
    *   `PgUp` / `PgDown` : Scroll logs page-by-page
    *   `Home` / `End` : Scroll to top / bottom of debug logs

### Settings View (`Mode::Settings`)
Full-screen configuration editor split into Categories and Fields.
*   *Shortcuts*:
    *   `Esc` : Return to category sidebar (if in fields panel) or return to Home Screen (if in sidebar)
    *   `q` / `Q` : Return to Home Screen
    *   `1` - `5` : Jump directly to Category 1-5 (General, Sorting, Scan, Theme, Keybindings)
    *   `←` / `h` / `H` : Focus category sidebar
    *   `→` / `l` / `L` / `w` / `W` : Focus fields panel on the right
    *   `↑` / `k` / `K` / `↓` / `j` / `J` : Navigate categories (if focused on sidebar) or fields (if focused on fields)
    *   `PgUp` / `PgDown` : Jump selection to top / bottom of active category or fields
    *   `Enter` : Edit selected setting field (or toggle switch)
    *   *Shortcuts when editing a field*:
        *   `Enter` : Save and commit changes
        *   `Esc` : Cancel edits and revert field value
        *   `Backspace` / character keys : Input value
        *   `↑` / `k` / `↓` / `j` / `PgUp` / `PgDown` / `Home` / `End` : Navigate items (only when selecting a theme)

### Signs & Symbols Legend Popup (`Mode::Legend`)
Quick reference guide to badges, operations, and status indicators.
*   *Shortcuts*:
    *   `Esc` / `ctrl-q` / `h` : Close legend popup
    *   `↑` / `k` / `↓` / `j` / `PgUp` / `PgDown` / `Home` / `End` : Scroll legend content

### About Popup (`Mode::About`)
Information about Gitwig, version details, and creator profile.
*   *Shortcuts*:
    *   `Esc` / `ctrl-q` / `V` : Close about popup

### Search Columns Selector Popup (`Mode::SearchColumnPicker`)
Picker to select columns to search by (SHA, Message, Author, Date) before entering fuzzy logs search.
*   *Shortcuts*:
    *   `↑` / `k` / `↓` / `j` : Navigate search columns (SHA, Message, Author, Date)
    *   `Space` : Toggle selected search column checkbox
    *   `Enter` : Confirm and open logs search input modal
    *   `Esc` / `q` / `Q` : Cancel and close columns selector

### Repository Settings Popup (`Mode::RepoSettings`)
Custom per-repository config editor (Theme, Page Size, Max Commits, Editor path, custom Note, Git LFS Tracking/Pull). Accessed via `s` on the repository Overview screen.
*   *Shortcuts*:
    *   `Esc` : Cancel text edit (if editing) or return to Detail view (if in popup)
    *   `q` / `Q` : Return to Detail view
    *   `↑` / `k` / `↓` / `j` : Navigate settings rows
    *   `←` / `h` / `→` / `l` : Change values for option fields (Theme, Resync on Tab Change)
    *   `Enter` / `Space` : Edit/Toggle setting (toggles themes, enters text input for fields, runs LFS Pull, or refreshes LFS details)

### Not Git Repository Popup (`Mode::NotGitRepo`)
Warning dialog shown when selecting a directory that is not a valid Git repository.
*   *Shortcuts*:
    *   `Esc` / `Enter` / `q` : Acknowledge warning and return to Home Screen

### Help Overlay (`Mode::Help` / `Mode::DetailHelp`)
Floating keyboard shortcut reference popup shown on top of the Home Screen or Detail View.
*   *Shortcuts*:
    *   `Esc` / `q` / `?` : Dismiss help overlay

### Global Search View (`Mode::GlobalSearch`)
Full-screen dashboard to search for keywords/code across all tracked repositories simultaneously.
*   *Shortcuts*:
    *   `Esc` : Clear search and return to Home Screen
    *   `Tab` : Toggle keyboard focus between search input and matches list
    *   `↑` / `↓` : Scroll/navigate matched items
    *   `PgUp` / `PgDn` / `Home` / `End` : Navigate matches list
    *   `Enter` : Trigger search (if input is focused) or open selected file match in Detail view (if list is focused)
    *   `Backspace` / character keys : Input query (when input is focused)

---

## 6. Interactive Search & Picker Overlays
Floating modal components with search-as-you-type inputs. Matches are ranked and updated in real time.

*   *Available overlays*:
    *   **RepoJump**: Fuzzy repository navigator (`/` on Home Screen)
    *   **RepoScanPicker**: Select repository path from scanned directories (`a` on Home Screen)
    *   **BulkAddScanPicker**: Select directory to scan and import repositories (`A` on Home Screen)
    *   **BranchSearchInput**: Fuzzy branch checkout picker (`/` on Branches Tab)
    *   **FileSearchInput**: Fuzzy file finder (`/` on Files Tab)
    *   **CommitFuzzySearch**: Fuzzy commit hash/message finder (`/` on Commits Tab or Logs View)
    *   **TagSearchInput**: Fuzzy tag checkout picker (`/` on Tags Tab)
*   *Shortcuts inside overlays*:
    *   `Esc` : Cancel and close overlay
    *   `Enter` : Confirm selection
    *   `↑` / `↓` : Navigate matched items list
    *   `Backspace` : Delete search query character
    *   Character keys : Input fuzzy search query query

---

## 7. Input Modals & Setup Dialogs
Modal text entry boxes for setting up configurations or typing messages.

*   *Available dialogs*:
    *   **Commit Message Dialog**: Type commit message and description.
    *   **Branch / Tag / Stash Creation dialogs**: Enter new names.
    *   **Import / Remote Setup dialogs**: Enter URLs, names, and paths.
    *   **Worktree / Submodule Setup dialogs**: Enter paths, branches, or lock reasons.
    *   **Label entry dialogs**: Prompt custom labels for repository grouping.
*   *Shortcuts*:
    *   `Esc` : Cancel / close dialog
    *   `Enter` : Submit entry (or `Ctrl+Enter` for multi-line inputs like the Commit Dialog)
    *   `Backspace` : Erase character
    *   Character keys : Input characters

---

## 8. Warning & Confirmation Modals
Double-checks before executing hazardous or state-altering actions.

### The Safety Intercept Feature
To prevent accidental data loss due to muscle-memory `Enter` keypresses, Gitwig intercepts keystrokes on the router level. For any modal classified as a **Destructive Action**, pressing `Enter` defaults to **Cancel** (`ConfirmNo`). To confirm a destructive action, the user must explicitly press `y` or `Y`. For non-destructive actions, `Enter` acts normally to confirm.

### Destructive Actions (`Enter` acts as Cancel)
*   **Discard Changes (`Mode::DiscardChangesConfirm`)**: Discarding unstaged/staged files, hunks, or lines.
*   **Delete Branch (`Mode::BranchDeleteConfirm`)**: Deleting a local or remote tracking branch.
*   **Delete Tag (`Mode::TagDeleteConfirm`)**: Deleting a Git tag locally or remotely.
*   **Delete Stash (`Mode::StashDeleteConfirm`)**: Dropping/deleting a stash entry from the stashes list.
*   **Delete Remote (`Mode::RemoteDeleteConfirm`)**: Removing a configured remote repository connection.
*   **Delete Submodule (`Mode::SubmoduleDeleteConfirm`)**: Deleting a submodule reference.
*   **Abort Merge (`Mode::MergeAbortConfirm`)**: Aborting an active merge session (reverting resolved conflicts).

### Non-Destructive Actions (`Enter` acts as Confirm)
*   **Branch Checkout (`Mode::BranchCheckoutConfirm`)**: Switching to another branch.
*   **Tag Checkout (`Mode::TagCheckoutConfirm`)**: Detaching HEAD to check out a tag.
*   **Branch Push (`Mode::BranchPushConfirm`)**: Pushing commits to remote.
*   **Tag Push (`Mode::TagPushConfirm` / `Mode::TagPushAllConfirm`)**: Pushing single or all tags to remote.
*   **Branch Merge (`Mode::BranchMergeConfirm`)**: Merging a branch into the active branch.
*   **Rebase (`Mode::BranchRebaseConfirm` / `Mode::BranchInteractiveRebaseConfirm`)**: Rebasing the current branch.
*   **Stash Apply (`Mode::StashApplyConfirm`)**: Applying a stash onto the working directory.
*   **Cherry-Pick (`Mode::CherryPickConfirm`)**: Cherry-picking a commit.
*   **Revert (`Mode::RevertConfirm`)**: Creating a revert commit for a specific commit.
*   **Continue Merge (`Mode::MergeContinueConfirm`)**: Continuing a merge after conflicts are resolved.
*   **Self-Update (`Mode::UpdateConfirm`)**: Confirming a self-update.

### Shortcuts
*   `y` / `Y` : Confirm action (required for destructive actions; works for all)
*   `Esc` / `n` / `N` : Cancel action / close popup
*   `Enter` : Confirm action (only works for non-destructive actions; cancels for destructive actions)

