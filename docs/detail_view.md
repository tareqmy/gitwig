# 🔍 Detail view

Press `Enter` on a selected item to open a full-screen Detail view. The detail view supports twelve tabs for git repositories, organized into two groups:

| Group | Tabs | Access / Navigation |
|---|---|---|
| **Primary** | **Workspace**, **Files**, **Graph**, **Branches**, **Tags**, **Remotes**, **Stashes** | Always visible. Use `1`–`7` to jump directly. |
| **Advanced** | **Worktrees**, **Submodules**, **Reflog**, **Forge Issues**, **Forge PRs** | Toggled via `Z`. Use `1`–`5` to jump directly when active. |

- Press `Z` from any tab to toggle between **Primary** and **Advanced** tab groups.
- Press `Esc` while viewing the Advanced group to step back to the Primary group.
- Press `Tab` / `Shift+Tab` to cycle forward/backward through the tabs *within* the currently active group.
- Directly jump to tabs using number keys:
  - In **Primary** group: Workspace (`1`), Files (`2`), Graph (`3`), Branches (`4`), Tags (`5`), Remotes (`6`), Stashes (`7`).
  - In **Advanced** group: Worktrees (`1`), Submodules (`2`), Reflog (`3`), Forge Issues (`4`), Forge PRs (`5`).
- Press `v` from any tab to toggle the repository **Overview** full-screen overlay.
- Alternatively, you can click on the visible tab headers directly with the mouse to switch tabs.
Press `Esc` or `q` (from the Primary group) to return to the repository list.

### Panel Layout & Navigation (Workspace Tab)

For a **git repository**, the **Workspace** tab is split into multiple rounded panels with a 40:60 width ratio on the bottom:
- **Commits (top 50%):** Lists the recent commits in the repository. If there are uncommitted changes, a special row named `Uncommitted changes` will be pinned to the very top.
- **Staging Area / Changed Files (bottom-left 40%):** Lists files that are modified, staged, or untracked. When `Uncommitted changes` is selected at the top, this panel is split vertically into `Staged` and `Unstaged` sections. When a real commit is selected, it is split horizontally, showing the list of files modified in that commit in the top half, and full commit details (hash, author, date, refs, and message) in the bottom half.
- **Staging Details (bottom-right 60%):** Displays the unified diff of the selected file.

### Files Tab

The **Files** tab displays all tracked files in the repository as an interactive directory tree on the left, and a preview panel on the right.
- Directory nodes are prefixed with `>` when collapsed and `▼` when expanded.
- File nodes are prefixed with `📄`.
- **Expand Folder:** Select a collapsed directory and press `>` or `.` or `Right-Arrow`.
- **Collapse Folder:** Select an expanded directory and press `<` or `,` or `Left-Arrow`.
- **Preview Panel:** Selecting a file displays its content (up to 100 KB) on the right; selecting a directory displays a list of files and folders directly inside it.
- **File History View:** Select any file in the tree and press `Shift+H` to open a history split-panel view, showing revisions on the left and the revision's diff on the right. Press `Tab` / `w` to cycle focus, and `Esc` / `q` to return.

### Graph Tab

The **Graph** tab renders the git log history graph / branch visualization graph.

### Branches Tab

The **Branches** tab is split vertically into left and right panels:
- **Local Branches (left):** Lists local branches in the repository, sorting the active branch to the top marked with ``.
- **Remote Branches (right):** Lists remote tracking branches.

You can focus either branch panel by pressing `w` / `W` or using the `←` / `→` arrow keys.

### Tags Tab

 the **Tags** tab lists both local tags and remote tags.
- Select a local tag and press `↵` (Enter) to checkout that tag.
- Select any tag and press `D` to delete it (asks for confirmation).
- Press `p` to push the selected tag to the remote (asks for confirmation).
- Press `P` (Shift+P) to push all tags to the remote (asks for confirmation).

### Remotes Tab

The **Remotes** tab lists configured remotes for the repository.

### Stashes Tab

The **Stashes** tab lists all available stashes in the repository with a horizontally split layout:
- **Stashes (Top-Left):** Lists all stashes. Selecting a stash automatically highlights the first file in the files list. Press `D` to delete the selected stash, or `a` / `A` to apply it (both options ask for confirmation). When applying, you can toggle whether to delete the stash after applying (default is Yes).
- **Stashed Files (Bottom-Left):** Lists the files changed/saved in the selected stash.
- **Stash Diff (Right):** Shows the unified patch diff of the selected file.
- Navigate stashes or stashed files using `↑`/`k` and `↓`/`j`.

### Stashing UI

Pressing `s` / `S` inside the **Workspace** tab opens the dedicated **Stashing UI** overlay popup:
- **Left Panel:** Displays the list of all modified, staged, unstaged, untracked, and conflicted files that will be stashed. You can navigate through this list using `↑`/`↓`/`k`/`j`.
- **Right Panel:** Displays checkboxes for stashing options:
- Toggle stashing untracked files with `u`.
- Toggle keeping the index with `i`.
- **Actions:**
- Press `s` to save the stash. This opens a text prompt to input an optional stash message/name (where stashing options can also be toggled with `Ctrl+U` and `Ctrl+I`).
- Press `Esc` / `q` / `Q` to cancel and return to the Workspace view.

### Worktrees Tab

The **Worktrees** tab lists all Git worktrees linked to the repository.
- Press `a` to add a new worktree. You will be prompted to enter a base branch/commit name, followed by the destination path.
- Press `l` to toggle lock status on the selected worktree (adds/removes lock reason).
- Press `D` to remove the selected worktree. You can choose to either remove only the Git metadata or delete the folder from disk entirely.
- Press `p` to prune stale worktree metadata.
- Press `Enter` to open the selected worktree (registers it to the Home screen cards list if not present, and opens it).

### Submodules Tab

The **Submodules** tab lists all git submodules defined in the repository:
- **Name:** The submodule identifier.
- **Status:** Shows `Clean`, `Modified` (local changes or commit mismatch), or `Uninitialized`.
- **Commit (Index):** The target commit SHA the superproject expects.
- **Commit (HEAD):** The actual checked-out commit SHA in the submodule directory.
- **URL:** The remote source URL of the submodule.

### Reflog Tab

The **Reflog** tab lists HEAD reflog entries for the repository, enabling easy recovery of lost commits and branches:
- **Selector:** The reflog reference index (e.g. `HEAD@{0}`).
- **Commit OID:** Shortened commit target hash.
- **Action:** The action type (e.g. `checkout`, `commit`, `rebase`, `reset`), highlighted based on the action.
- **Message:** The action details and commit message.
- **Time / Date:** Relative age and absolute UTC date of the operation.
- **Checkout Commit:** Press `Enter` or `Space` on any entry to checkout that commit hash.

### Overview Overlay

Pressing `v` from any repository detail tab displays the **Overview** overlay, showing resolved paths, tracking info, and committer statistics. Press `Esc`, `q`, or `v` to return to your workspace. Press `s` inside the Overview overlay to open the Repository Settings page.

### Navigation & Interaction

You can navigate and interact with these panels in the following ways:
- **Cycle Focus:** In the Workspace, Files, Branches, Tags, Stashes, Worktrees, Submodules, and Reflog tabs, press `w` / `W` to cycle panel focus:
- **Workspace tab:** `Commits` → `Staged` → `Unstaged` → `StagingDetails`.
- **Files tab:** `Files` (left tree list) ↔ `FileContent` (right preview panel).
- **Branches tab:** `Local Branches` ↔ `Remote Branches`.
- **Tags tab:** `Local Tags` ↔ `Remote Tags`.
- **Stashes tab:** `Stashes` → `Stashed Files` → `StagingDetails`.
- **Worktrees tab:** `Worktrees` (main table).
- **Submodules tab:** `Submodules` (main table).
- **Reflog tab:** `Reflog` (main table).
Focus defaults to the main panel of the tab when switching tabs (e.g., `Commits` on Workspace tab, `Files` on Files tab, `Local Branches` on Branches tab, `Local Tags` on Tags tab, `Stashes` on Stashes tab, `Worktrees` on Worktrees tab, `Submodules` on Submodules tab, `Reflog` on Reflog tab).
- **Mouse Click to Focus/Select:** Left-click inside any panel's boundaries (including branch/tag/stash list panels, stashed files list, files list, and the files tab content preview panel) to focus it immediately.
- **Resize Split Panels:** Left-click and drag the vertical or horizontal boundary splitter lines between panels to resize them dynamically. This is supported in:
- **Workspace / Inspect:** Main vertical split (commits vs details), bottom horizontal split (left list vs right diff), and left vertical split (staged vs unstaged or commit details vs files list).
- **Files:** Horizontal split (repository files tree vs file content preview).
- **Branches:** Horizontal split (local branches vs remote branches).
- **Stashes:** Horizontal split (stash lists vs diff) and left vertical split (stashes list vs stashed files).
- **Overview Overlay:** Horizontal split (overview info vs committer stats).
- **Mouse Wheel Scroll:** Use the mouse wheel to scroll vertically through the active list, commit history, branch list, files list, stashed files list, staging details diff, or files tab preview panel.
- **Navigate Lists:** Use `↑`/`k` and `↓`/`j` to select a commit, file, branch, tag, stash, stashed file, or file tree item in the active list.
- **Scroll Diff:** When the `Staging Details` panel is focused, you can scroll the unified diff text vertically using `↑`/`k` and `↓`/`j` (line-by-line) or `PgUp`/`PgDn` (page-by-page).
- **Scroll File Content:** When the files tab `FileContent` preview panel is focused, you can scroll the preview text vertically using `↑`/`k` and `↓`/`j` (line-by-line) or `PgUp`/`PgDn` (page-by-page).
- **Stage/Unstage Files:** Select the `Uncommitted changes` row at the top, select a file in either the `Staged` or `Unstaged` list, and press `Enter` to stage or unstage that file instantly. Press `a` to Stage All (when focused on Unstaged) or Unstage All (when focused on Staged), which automatically shifts panel focus to the opposite list. Press `x` to discard changes in the selected file, or `X` to discard all changes in the repository (both ask for confirmation before performing).
- **Network Progress Bar:** Any long-running network operations (such as Fetch, Pull, or Push) will display a centering animated progress bar popup so the UI thread remains responsive and visual feedback is clear. If an operation hangs or takes too long, you can press `Esc` to force-dismiss the progress popup and resume TUI interaction immediately.
- **Multi-Remote Selection Picker:** For write operations (pushing branch/tags, deleting tags, fetching tags), if the repository has multiple configured remotes and no upstream tracking branch is set, a selection picker popup will appear allowing you to select which remote to target using ↑↓ and Enter.
- **Manage Branches:** While on the **Branches** tab:
- Select any **local branch** (other than the currently active one) and press `Enter` to check it out (safe checkout strategy applies).
- Press `f` / `F` to fetch updates from the remote in the background.
- Select any **local branch** and press `p` to pull updates from its remote in the background (can only pull into the currently checked-out branch).
- Select any **local branch** and press `Shift+P` to push it to the remote in the background (if no upstream is configured, it will attempt to push to the first configured remote and set it as the upstream tracking branch).
- Select any **remote branch** and press `Enter` to check it out (this will switch to the branch, creating a local tracking branch if it doesn't already exist).
- Press `c` / `C` to create a new branch from the currently checked out branch (or HEAD). A popup dialog will prompt for the new branch name.
- Select any local or remote-tracking branch and press `D` to delete it. A confirmation dialog will prompt before performing the deletion. (The currently checked out branch cannot be deleted).
- **Commit Staged Changes / Amend Last Commit:** Press `c` from the Workspace tab or from the Inspect view to open a centered Commit popup window (active if there are staged changes OR a prior HEAD commit exists to amend). 
- **Compose Mode:** Type your commit message. Press `Ctrl+C` to lock in the text and switch to confirmation state, press `Enter` to insert a newline, or `Esc` to cancel.
- **Confirm Mode:** Press `Enter` to execute the commit, `e` to return to composing/editing the message, `a` / `A` / `Space` to toggle the "Amend last commit" option, or `Esc`/`q` to close the popup. Toggling amend to active when the buffer is empty automatically populates the text box with the message from the last commit.

For a **plain directory** the view confirms the resolved path and explains that no `.git` entry was found.

For a **missing** item the view confirms the resolved path and notes that the path does not exist or isn't accessible.

The detail snapshot is taken **once** when you press Enter — it is not refreshed while open (except for staging/unstaging actions which refresh dynamically). Close and re-open to re-read the repository state.

After every successful add / edit / delete, the status bar briefly shows `Saved` or `Deleted`. If the write fails, the status bar shows `Save failed: <reason>` instead — your in-memory list still reflects the change, but the file on disk does not.

### 🔀 Main Page Sorting

The main page repository list can be sorted dynamically using shortcuts:
- **Cycle Sort Order (`o`):** Cycles through the sorting modes: `Custom` (preserves your manual list order), `Alphabetical`, `Recent Visit` (tracks when you enter a repository's detail view), and `Latest Changes` (based on HEAD commit timestamps or folder modification times).
- **Toggle Sort Direction (`O`):** Toggles ascending vs. descending/reversed sort direction. The top border displays the active sort mode (e.g. `Sort: Alphabetical (Rev)`).
