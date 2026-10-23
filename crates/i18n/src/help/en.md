## Quick navigation
- `↑↓` selects an item; `PgUp` / `PgDn` scrolls a page.
- With empty input, `Enter` opens the selected item; `Esc` goes back.
- With text, `←→` moves the cursor; `Esc` clears input first.
- `/repos`, `/settings`, and `/schedule` open repositories, general settings, and the scheduled task; `Tab` does not switch these pages.
- `F1` / `/help` opens this page; `Ctrl+C` / `/quit` exits.

## Mouse & Trackpad
- General settings has a Mouse & Trackpad group with independent precise-click, scrolling, and pane-selection toggles. All default to on and apply immediately after saving.
- Single-click an item to select it; double-click to perform its Enter action, keeping existing confirmations. Scrollbar clicks only seek. Titles and blank space never activate Enter. Disabling precise clicks also disables double-clicks.
- With pane selection off, clicks and scrolling only affect the focused pane. Disabling wheel input leaves keyboard navigation available.

## Repository panes
- Breadcrumbs start at Repositories. Opening a child view adds a level. Click an ancestor to return with selection and search preserved; this is disabled during command editing or confirmation.
- Focusing a pane adds one level; switching panes replaces it. Maximizing adds no level. Back at a pane root returns to the repository root, then to the home page.
- Entering a repository shows overview, branches, worktree files and the commit graph together.
- `/repo branch` always opens the full branch view. Type to search names; `Tab` / `Shift+Tab` cycles All / Local / Remote.
- In branches, `Enter` opens ref, commit and upstream details. `*` marks the current branch. Ahead/behind uses local refs without fetching or changing branches.
- `Ctrl+S` / `Ctrl+D` focuses the previous / next pane. Clicking or scrolling inside a pane also focuses it. At the repository root, Enter / → or Ctrl+D enters Overview; Ctrl+S enters History.
- `Ctrl+F` toggles the focused pane between fullscreen and the tiled layout.
- Switch panes while maximized without losing selection, filters or scroll positions. Input and commands stay available.
- At a pane root, Esc leaves focus and restores the tiled layout, just like clicking the repository breadcrumb. Maximizing adds no extra Back step. Esc clears input first; diff and commit details return to their pane first.

## Search and commands
- Type to search repositories, or files in the file panel. Input is always active.
- With empty search, `/` opens this view's full-width command menu. `↑↓` selects, `Tab` completes, Enter opens. `//` opens global commands directly.
- Filter commands by name or Chinese/English descriptions, ignoring English case; e.g. `//live diff` or `//终端`. Name matches come first; Tab completes the selected command.
- The global menu shows top-level entries. A prefix such as `/repo diff` also shows all matching subcommands; no trailing space is needed. Parent commands come first.
- Parameter commands stay in the input: `/repos add <path>`, `/settings concurrency <count>`.
- `/repo diff view` switches between the directory tree (default) and flat list; use `/repo diff view tree` or `/repo diff view flat` directly. Single-child directory chains appear together (for example, `output/imagegen/`). Directories show hierarchy; selection and preview act on files.
- `Ctrl+U` clears text before the cursor; `Ctrl+A` / `Ctrl+E` moves to the start / end.
- On the repository home page, `/repos filter` and `/repos sort` open their options: ↑↓ selects, Enter applies, Esc closes. ✓ marks the current option.
- Filters: all, problems (attention or failed), clean, failed, unchecked, no-upstream, pushes (recorded successes or failures).
- Sorting: priority, name (A to Z), path (ascending), updated (latest state first; unchecked last). Direct commands work too, e.g. `/repos sort name`.
- Filtering combines with search. Press `/` on the home page to open commands while keeping an existing search; applying or closing returns to that search and keeps the selected repository when visible. Options apply only to this session and do not check repositories or change reports.
- `Ctrl+R` reloads the report or refreshes the file panel. The mouse wheel scrolls three rows in the repository list.

## Context prefixes
- `/repo diff`, `/repo log`, `/repo branch` and `/repo stash` open full views. Back restores the previous focus and selection. Pressing `/` in a pane only opens its command context. Settings, schedule and help also restore their origin on return.
- The view prefix is a separate locked token: /repo diff for files and diffs, /repo log for history, /repo branch for branches, and /repos, /settings, /schedule for management pages. Home opens global / commands; overview uses /repo and stash uses /repo stash.
- Type only the operation suffix, e.g. stage, unstage, layout or refresh in a worktree diff. Name and bilingual description matching stay within the current group.
- With an empty suffix, Backspace selects the whole prefix, using its original selection colors. Candidates match the global / menu; use ↑↓ to select, Tab to complete or click a candidate to replace the prefix. Typing or pasting replaces it with a global command while retaining one leading /. Enter or → keeps the original prefix without running a command; a second Backspace closes commands and restores the current view’s search. Esc closes commands and restores search.
- Another / with an empty command suffix switches to global commands too; typing //settings from search opens global settings directly.
- Switching command ranges does not leave the view or change repositories. The context stays fixed while editing; Esc restores search. Reopening captures the current focus again.
- With existing search, Home then / opens commands while retaining search. Slashes inside a search path remain literal text.
- Historical diffs offer layout and copy operations without worktree staging or refresh. Directory completion, configuration review and layout-picker cancellation retain context, drafts and cursor positions.

## Copy the current selection
- Press `/` in a view and type `copy`. Candidate labels identify the content. There is no global copy group; `/repo copy` copies the selected repository's full path from home.
- In file lists, `/repo diff copy` copies marked file paths or the cursor file when none are marked. Paths are relative, deduplicated and separated by newlines; Unicode and spaces stay literal.
- In a focused diff preview, `/repo diff copy` copies the loaded original patch without UI line numbers, colors or wrapped lines. It never rereads Git.
- `/repo log copy` copies the selected full commit ID in the graph, or the selected file path in commit details. Historical diffs use `/repo diff copy`.
- `/repo branch copy` copies the selected branch name. In operation errors, `/` opens /error and copy copies complete error details.
- Combined panes follow focus. Failed previews or panes copy their error details. Opening commands freezes the repository and selection; background refresh cannot replace the target.
- Images, loading, empty and truncated patches report explicit errors. Clipboard writes run independently, report success after the system accepts them and preserve navigation and new drafts.
- Linux needs X11 or a Wayland session supporting data-control. Content retention after exit depends on the desktop clipboard manager. Error pages accept only copy and quit commands.

## Paths and completion
- `Tab` completes the directory component at the cursor. One match adds `/`; multiple matches show directory candidates.
- In candidates, `↑↓` / `Tab` / `Shift+Tab` selects; `Enter` fills the directory and keeps editing; `Esc` closes the menu.
- `~` and `~/…` use your user home, independent of the application's data directory. Review and execution use absolute paths.
- Spaces and Chinese text are literal. Matching single or double quotes may enclose the entire path, including `~/…`.
- Hidden directories appear after a `.` prefix. Completion reads one directory level and retains the path after the cursor.
> A path is the entire remaining argument. No variable, wildcard or shell escape expansion occurs. Named-user paths such as ~user are unsupported.

## Branches, fetching and stashes
- `/repo branch create <name>` creates a local branch at the current commit without switching. `/repo branch switch <name>` switches to an existing local branch. Tracked files must be clean; untracked files are protected, and branches checked out in another worktree are refused.
- `/repo fetch` fetches configured remote-tracking branches immediately in the background, honoring refspecs, exclusions and skipFetchAll. It does not change the worktree or push.
- `/repo pull` fetches once and reviews a pinned upstream commit. Confirmation allows fast-forward only; divergence and dirty worktrees are explicit errors. No automatic merge, rebase or stash.
- `/repo stash` opens a searchable list. Use ↑↓ to select, Enter for details, and ←/Esc to return. Slash commands use the /repo stash context; search matches messages and object IDs.
- `/repo stash save <message>` saves tracked changes; `/repo stash save-all <message>` also includes untracked files. Ignored files are excluded.
- `/repo stash apply <stash-id>` restores and retains the entry. `/repo stash pop <stash-id>` removes the entry only after successful restoration. `/repo stash drop <stash-id>` removes it without restoring. Completing these commands from the list fills the selected full object ID for review.
- `/repo stash copy` copies the selected full object ID; `/repo stash refresh` reloads the list. Restore conflicts are reported as failures and retain the original stash and actual conflict state.
- Writes require review and confirmation; returning to edit preserves the draft. HEAD, index, worktree and target references are revalidated. Stash operations also validate reflog order; external changes require a new review.

## Files, commits and pushes
- `/repo diff` opens one changes workspace: unstaged and staged file groups plus a live text or image preview. Choosing a file loads its exact comparison in the background.
- Wide windows show files beside the preview; narrow windows stack them. `Tab` / `Ctrl+S` / `Ctrl+D` switches focus; `Enter` / `→` focuses the preview, and `Ctrl+F` maximizes the focused pane.
- `Esc` first clears input, then restores a maximized pane, then returns focus to files. Another `Esc` returns to the repository pane or list.
- Diff defaults to side-by-side on wide screens and unified on narrow screens. Long lines stay on one row; both sides scroll together with fixed line numbers.
- `/repo diff layout` opens a horizontal selector at the current setting. `←→` / `Tab` / `Shift+Tab` chooses; `Enter` applies; `Esc` restores the command draft. Clicking an option selects it without applying.
- Command candidates show the current setting; Auto also identifies the visible unified or split layout and updates on resize. Click a command row or press `Enter` to open it.
- `/repo diff layout auto`, `/repo diff layout split`, `/repo diff layout unified` changes layout immediately for this TUI session. `Tab` completes these subcommands.
- Click a file checkbox to toggle its selection; click its name to select the row, or double-click its name to enter. This follows the precise-click preference. `Space` marks a file with empty input; `Insert` always marks; `Ctrl+A` marks all visible files. Filtering retains hidden selections.
- `/repo diff stage` / `Alt+S` stages marked files or the cursor file; `/repo diff unstage` / `Alt+U` unstages them. These actions require an explicit file selection and never open a page.
- Stage accepts only unstaged files; unstage accepts only staged files. Mixed selections report an error without changing the index. Completion refreshes only this repository and preserves focus and new command drafts.
- Unstaging retains worktree contents. Git-ignored files are not listed.
- `/repo commit` previews staged contents, then takes a message and confirms. Git hooks and signing are preserved.
- `/repo push` fetches and validates the upstream, then asks for confirmation. Only the reviewed branch snapshot is pushed.
- `/repo terminal` opens the system default terminal in the selected repository; type `/te` then `Tab` to complete.

## Image previews
- Selecting a PNG, JPEG or WebP in `/repo diff` automatically previews both versions; in commit details, press `Enter` to compare. Content detection does not require an extension.
- Wide windows show versions side by side; narrow windows with sufficient height stack them. Short panes retain side-by-side images. Images keep their aspect ratio and use a checkerboard for transparency.
- `Esc` returns; resizing fits images to the pane. Missing versions, corrupt images and unavailable pixel support are explicit.
- Index and historical images come from Git objects, never from current worktree contents.

## Commit graph
- `/repo log` reads local branches, remote-tracking branches, tags and HEAD without fetching.
- `↑↓` / `PgUp` / `PgDn` selects; `Enter` opens commit details and then a historical file diff; empty-input `Esc` goes back.
- Each page contains up to 100 commits; `↓` at the bottom or `/repo log more` loads the next page. `Ctrl+R` refreshes the reference snapshot.
- Type to locate loaded commits by hash, title, author or ref. Nodes remain visible; `Tab` / `Shift+Tab` selects the next / previous match.
- `Alt+←→` pans a wide graph. Wide terminals show a metadata preview.
- In commit details, type to filter files, `Tab` switches merge parents and `PgUp` / `PgDn` scrolls metadata and message.
- Root commits compare with the empty tree; other diffs identify the parent and selected commit. `/repo diff` layout commands also apply.
> Graph and historical diff reads do not stage, commit, fetch, push or change reports.

## Checks and configuration
- `/repo check` checks the selected repository; `/repos check` checks every registered repository. Both start directly without pushing.
- Checks run in the background. Browse, search and view files while they run. A spinner marks active checks; unfinished rows retain previous results.
- `/cancel` cancels the selected repository task, or the batch check when no selected task exists. Independent repositories run concurrently; operations sharing Git storage serialize. Configuration writes use a short transaction.
- `/repo` opens the selected repository overview. All repository views and operations live under it. `/repos` returns to the repository list; `/repos manage` opens registration and exclusion settings. Direct commands: `/repos add <path>`, `/repos remove`, `/repos exclude <path>`, `/repos unexclude <path>`.
- `/repos scan <path>` previews discovery; `/repos import <path>` registers discovered repositories. Scanning follows Git ignore rules.
- `/settings concurrency <count>` sets bounded concurrency, from 1 to 32.
- `/settings startup-check <true|false>` controls startup checks of all repositories; default true, effective next launch.
- `/schedule` opens task management: status, pending interval, configuration and enable / disable controls.
- `/schedule interval <duration>` accepts values such as 6h; `/schedule autopush <policy>` accepts disabled or fast_forward.
- `/schedule anchor <datetime|auto>` sets a local time basis (HH:MM[:SS] or YYYY-MM-DD HH:MM[:SS]); auto regenerates it on enable.
- The first enable persists an automatic basis when omitted. Restarts and re-enabling preserve it; the schedule page shows the basis and applied next trigger.
- Durations support s / m / h / d / w (at least 60 seconds). Apply saved changes with `/schedule apply`.
- `/schedule apply` applies configuration; `/schedule enable` enables scheduling; `/schedule disable` stops future runs.
> Automatic pushing applies only to scheduled checks. The TUI checks all repositories in the background on startup by default; disable this in general settings. It does not scan or push. Completed startup checks update the batch report. Stage, commit and push refresh only that repository from local Git state, without another fetch.

## Confirmation
- `←→` / `Tab` selects an action; `Enter` confirms; `Esc` returns. Returning from commit or configuration confirmation retains the draft.
- Commit messages use the bottom input. Repository reads and writes run in the background; navigation and command input remain available. Completion updates its repository and does not replace another view or a new command draft.
- `/cancel` cancels repository work; `Esc` returns through views. `Ctrl+C` exits, cancelling and collecting owned tasks and Git processes. Configuration tasks use `Esc` to cancel.

## Status legend
- `✓` Clean     `!` Needs attention     `×` Check failed     `·` Unchecked
- `*` Worktree changes: uncommitted, untracked or submodule changes. `?` means unknown.
- The list merges per-repository versions from checks and local operations. Newer repository results win over late batch results. The file panel reads current local state.
- No-upstream branches are counted separately. They are not problems and are never published automatically.
- Sync combines remote checks and push results. Failures and pending changes take priority; push counts refer to branches.
- Full errors appear in preview and repository details.

## Scrolling indicators
- Scrollbars support clicking the track and dragging the thumb on both axes. Dragging stays with the original pane until release, including outside the track; the precise-click preference applies.
- Vertical scrollbars sit in the right border over content rows only, excluding fixed headers. Unstaged and staged groups have separate tracks; content that fits has no scrollbar.
- Use `Ctrl+S/D` to switch panes; arrow keys do not switch panes.
- In a diff preview with empty input, ←→ scrolls long lines. Line numbers and change markers stay fixed; split columns pan together. Esc goes back. Overflowing content has a scrollbar embedded in the bottom border without taking a content row.
- In history, ←→ scrolls refs and subjects while graph lanes and short hashes stay fixed. Alt+←→ pans the graph, Enter opens details, and Esc goes back.
- Horizontal wheel and Shift + vertical wheel also work when forwarded by the terminal. With text in the input, arrow keys edit the input; command and confirmation selectors keep their own keys.

## Language
- System Chinese uses Chinese; all other languages use English.
- Set `APP_LANG=en` or `APP_LANG=zh` before starting to override the system language.
