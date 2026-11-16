## Quick navigation
- Run `gw` / `gw tui` from a Git root, subdirectory or worktree to open its repository details; other directories open the repository list. `gw repos` always starts at the list. Unregistered repositories stay session-only and are not added to scheduled workflows.
- `↑↓` selects an item; `PgUp` / `PgDn` scrolls a page.
- With empty input, `Enter` opens the selected item; `Esc` goes back.
- With text, `←→` moves the cursor; `Esc` clears input first.
- `/repos`, `/settings`, and `/schedule` open repositories, general settings, and the scheduled task; `Tab` does not switch these pages.
- `F1` / `/help` opens this page; `/quit` exits immediately. `Ctrl+C` clears nonempty command/search input, including a command prefix. When input is already empty, it immediately shows an exit prompt; the next press quits. Other keys, paste, clicks or scrolling dismiss the prompt. Separate commit-message and prompt editors retain their contents.

- Display → File icons controls type icons in the browser, worktree, and commit file lists. It is enabled by default and saved as `file_icons`. Nerd Font glyph support is required; disabling icons preserves tree indentation and browser name alignment.

- External saves, staging, commits and branch changes automatically refresh local views. Native file observation failures appear in the footer. Refreshing does not contact remotes or run worktree operations.

- Recovery actions reuse the bottom input bar; its title shows the current operation and conflict count. Opening a conflicted file selects Conflict actions; staging all conflicts selects recovery actions. Lists default to input. Manual page choices within the same context and active drafts take priority over refreshes. Shift+Tab switches between input and options; ←→ selects Continue, Skip or Abort, Enter opens review, and Esc returns to content. Typing resumes file search, / opens commands, and switching retains the draft and layout height. Continue stays disabled until all conflicts are resolved and staged.
- Conflicted files show left (current), right (incoming) and the full working source in one workspace. `/repo diff conflict [overview|include-base|base-left|base-right|base-result|left-right|left-result|result-right]` selects the display. `overview` shows both inputs above the result; `include-base` adds base at the top. The other six choices compare only the named versions side by side in that order. The file list, breadcrumbs and input stay in place. Tab completes; Enter applies. `/focus` changes regions. Each pair aligns its own sources and retains its reading position. Refresh updates the result; Esc follows normal navigation. Missing, binary and oversized sources are explicit. `/editor` edits the worktree; stage before continuing recovery. `/copy` follows focus and copies visible versions or original working source.
- `/repo diff conflict accept [left|right|both]` resolves the selected block; both keeps left before right. `conflict block [previous|next]` changes blocks. Titles show the block number and line numbers mark its region; accepting advances to the next block. `conflict undo` undoes the last resolution or reset in this file. `conflict reset` reviews and recreates all conflict blocks from index inputs, including after reopening the file; cancellation is selected by default. Resolve and stage the recreated result before continuing. Accepting changes only that block; stage the finished result before continuing. External edits or changed Git state reject a stale write. Use `/editor` for unsupported marker content. Use Shift+Tab to cycle Input, Conflict actions, and Git operation pages; the title shows the page number. In Conflict actions, use Left/Right to choose left, right, both, undo, reset, or stage; Enter runs it. Up/Down select blocks, Home/End select first/last, and PageUp/PageDown or the wheel scroll. Tab remains completion; Esc restores the input draft.

## Mouse & Trackpad
- `Ctrl+G` toggles Interactive / Text selection; global `/settings interaction` opens choices (`//settings interaction` from a context). `/settings interaction interactive` and `/settings interaction text` switch directly. The preference persists across restarts and is also in Settings. Text selection releases pointer input to the terminal for selection, Copy and context menus; application clicks, scrolling and dragging are disabled without changing pointer preferences. Keyboard navigation, typing, paste and commands remain active; Esc goes back, Ctrl+C uses the input-clearing and exit-confirmation rules above. Display updates and background tasks continue.
- General settings has a Mouse & Trackpad group with independent precise-click, scrolling, and pane-selection toggles. All default to on and apply immediately after saving.
- Single-click an item to select it; double-click to perform its Enter action, keeping existing confirmations. Scrollbar clicks only seek. Titles and blank space never activate Enter. Disabling precise clicks also disables double-clicks.
- With pane selection off, clicks and scrolling only affect the focused pane. Disabling wheel input leaves keyboard navigation available.

## Command targets
- Repository actions show the captured repository and branch, resolved locally in the background. Long paths elide the middle while preserving file names; operations retain the full original path.

## External editors
- `/editor` opens the selected repository, working file or folder. The menu shows the captured target; historical content, settings and empty lists have no editable target.
- `/settings editor` lists detected editors. `/settings editor git` follows the current repository's Git editor. You can also enter a command such as `/settings editor code --reuse-window`. This saves the preference without modifying Git or shell configuration.
- Git resolves `GIT_EDITOR`, `core.editor`, `VISUAL`, `EDITOR`, then its compiled default. GUI editors open independently; terminal editors return to the existing view on exit. Historical file previews do not offer editing.
## Common local actions
- `/terminal` opens the selected folder, the containing directory of a selected file, or the repository root in reference views. The menu shows the captured destination.
- `/refresh` and `Ctrl+R` refresh local data for the current view. Repository lists reload saved reports; file browsing reloads directories and preview; worktree, branches, history and stash lists reread local data. Historical snapshots cannot be refreshed.


## Repository panes
- Breadcrumbs start at Repositories. Opening a child view adds a level. Click an ancestor to return with selection and search preserved; this is disabled during command editing or confirmation.
- Focusing a pane adds one level; switching panes replaces it. Maximizing adds no level. Back at a pane root returns to the repository root, then to the home page.
- At the repository root, arrows select panes by layout. Enter focuses the selected pane before operating its content. A border marks the selection; focusing adds the pane name to breadcrumbs. Esc returns to pane selection and keeps its position; another Esc returns home. At the left column, ← also returns home; from the right column it first moves left. Narrow windows show the selected pane.
- Entering a repository shows overview, branches, worktree files and the commit graph together.
- `/repo branch` always opens the full branch view. Type to search names; `/repo branch filter [all|local|remote]` selects All / Local / Remote.
- In branches, `Enter` opens ref, commit and upstream details. `*` marks the current branch. Ahead/behind uses local refs without fetching or changing branches.
- `/focus` enters global pane focus mode in repository grids, diff and file browsers. Arrows move by layout without wrapping; Enter/Esc finish. Search pauses and arrow keys navigate panes while the input clearly shows Focus mode; / opens global commands. Use //focus or //zoom from a command context. Finishing restores type-to-search and preserves each pane’s selection and position. Clicking or scrolling inside a pane also focuses it.
- `/zoom` toggles the focused pane between fullscreen and the tiled layout.
- Switch panes while maximized without losing selection, filters or scroll positions. Input and commands stay available.
- At a pane root, Esc leaves focus and restores the tiled layout, just like clicking the repository breadcrumb. Maximizing adds no extra Back step. Esc clears input first; diff and commit details return to their pane first.

## Cherry-pick and revert
- In history or commit details, `/repo log cherry-pick` applies the selected commit to the current branch; `/repo log revert` creates a new inverse commit. Use `cherry-pick` or `revert` in the history context. Cherry-pick supports multiple selected commits; revert acts on the current commit.
- Commit checkboxes are hidden by default. Run `/repo log select` to enter selection mode, then click a checkbox, press `Space` with empty input, or press `Insert`. `select toggle` toggles the current commit, `select all` marks loaded matches, and `select clear` clears marks while staying in selection mode. `select done`, or `Esc` with empty input, exits selection and clears marks. Use `Shift+↑↓` to extend or shrink a range without removing earlier individual marks; ordinary movement starts a new range. `Enter` opens actions for marked commits: cherry-pick, copy hashes, clear marks, and exit selection; a single marked commit also offers revert. Use arrows then `Enter`, or click an action. Repository changes still require a preview and confirmation. `Esc` returns to selection; an empty selection prompts for marks. Outside selection mode, `Enter` still opens details. Typing still locates commits. Marks survive preview return and pagination; refresh exits selection. Batches run oldest first and contain at most 512 commits.
- Review the target branch, source commit and files before executing. Batches use Git’s native sequencer and retain remaining commits after conflicts. Replay merge commits separately; they require an explicit mainline parent; the operation uses the difference from that parent to the merge commit, inverted for revert.
- Conflicts open the changes view, with red `U` files, unresolved and remaining counts, and Continue, Skip, and Abort buttons. Resolve and stage them, then run `/repo continue`. `/repo skip` discards the current commit's conflict resolutions and skips it; `/repo abort` discards this operation and restores its starting state. Both require confirmation.
- Recovery commands appear only while cherry-pick or revert is in progress, including after restart. Completing or aborting the sequence marks its matching pending tasks handled while retaining their history.

## Search and commands
- Command rows show names, descriptions and current values. After Tab/Enter confirms a parameter command, a muted ghost hint appears at the cursor; fixed choices still open their candidates. Hints never enter the draft, submitted arguments or history. Typing a value hides them; editing inside the draft hides them until the cursor returns to an empty parameter at the end.
- With only the global `/` in the input, press `↑` at the first command for history. With a context prefix, arrows only select subcommands. Ordinary lists keep `↑` navigation. Choose with `↑↓`; `Enter`/`Tab` fills the draft, then `Enter` runs it. `Esc` returns to commands. The latest 128 unique submissions persist per data directory; searches and completions are not recorded.
- When opening a command group, direct entries precede deeper actions; typing still searches all descendants. Temporarily unavailable commands and options keep their positions and show a muted reason; Enter or clicking preserves the draft without starting work. This includes missing commits or operations and an already active display mode. Worktree actions in historical diffs, preview switches for unsupported files, and disabled AI features stay hidden. Selection actions show their file, line or commit count.
- `/` at the start of the input opens commands; `/` and `、` inside a search remain literal path characters. Pasting a complete `//command` opens global commands from an existing search; closing the menu restores that search.
- Command, option and directory candidates support `PgUp` / `PgDn` using the visible row count, stopping at either end. After closing options, `Enter` with an empty parameter reopens them without applying a default.
- Use commands for pane focus, zoom, staging and graph panning. Typing searches immediately; basic navigation and editing keys remain. No terminal configuration changes are needed. Alt+B/F moves by words, Alt+D deletes the next word, and Alt+Left/Right also moves by words in command and commit editors.
- The command menu sits directly above the input, followed by key hints and one reserved task-status row. Success notices disappear after 5 seconds; pending failures and running tasks remain. Use `/tasks` or click the status row to open tasks and return to the original page and input draft.
- Tasks show pending failures, running work, and the latest 200 completed records, retaining all active and pending tasks. `Enter` opens full details; `c` requests cancellation, `r` retries manual checks or fetch, and `d` marks a failure handled. Viewing a failure does not dismiss it. A successful retry resolves its original failure. Scheduled checks share this history.
- Type to search repositories, or files in the file panel. Input is always active.
- `Shift+Tab` switches the bottom bar between input and options when available. `Tab` completes commands and parameters. Options use arrow keys and Enter; Esc or typing resumes input. Drafts and layout height are retained. Use `/focus` to switch content panes.
- With empty search, `/` opens this view's full-width command menu. `↑↓` selects, `Tab` completes, Enter opens. `//` opens global commands directly.
- Filter commands by name or Chinese/English descriptions, ignoring English case; e.g. `//live diff` or `//终端`. Name matches come first; Tab completes the selected command.
- The global menu shows top-level entries. A prefix such as `/repo diff` also shows all matching subcommands; no trailing space is needed. Parent commands come first.
- Parameter commands stay in the input: `/repos add <path>`, `/settings concurrency <count>`.
- `[tree|flat]` means an optional fixed choice; `<path>` or `<name>` accepts user input. Inline hints use the available width and elide with `…` between whole options; parameter candidates list the complete choices. `[<model>|default]` accepts a model name or the special value `default`.
- `/repo diff view` switches between the directory tree (default) and flat list; use `/repo diff view tree` or `/repo diff view flat` directly. The choice is saved and restored on restart. Single-child directory chains appear together (for example, `output/imagegen/`). Directories can be focused and checked; files can be previewed.
- `Ctrl+U` clears text before the cursor; `Ctrl+A` / `Ctrl+E` moves to the start / end.
- On the repository home page, click the Filter or Sort label or its current value to open the choices. `/repos filter [all|problems|…]` and `/repos sort [priority|name|…]` open the same menus. The current value is selected; choosing an option applies it. Filter by name or bilingual description; ↑↓ selects, Tab completes, Enter applies, and Esc closes the choices. ✓ marks the current value. Use commands when precise clicks are disabled or text-selection mode is active.
- Auto-push policy and the AI agent use the same parameter menu: Space or Enter opens choices, ↑↓ selects, Tab only completes, and Enter submits the value. Existing confirmation rules still apply; cancelling does not save.
- Filters: all, problems (attention or failed), clean, failed, unchecked, no-upstream, pushes (recorded successes or failures).
- Sorting: priority, name (A to Z), path (ascending), updated (latest state first; unchecked last). Direct commands work too, e.g. `/repos sort name`. Sorting is saved and restored on restart.
- Filtering combines with search. Press `/` on the home page to open commands while keeping an existing search; applying or closing returns to that search and keeps the selected repository when visible. Filters and searches apply only to this session and do not check repositories or change reports.
- `Ctrl+R` reloads the report or refreshes the file panel. The mouse wheel scrolls three rows in the repository list.

## Filter and locate
- Global `/search` lists the modes supported by the current view and marks the current value. `/search filter` shows matching rows; `/search locate` retains the full list. Use `//search` from a view with a context prefix.
- Repository, worktree, branch, stash and historical file lists support both, defaulting to filtering. Switching preserves the query and any still-visible selection. Each list remembers its mode for this session.
- Locate retains the full list. Matching text has an amber background and underline; the current match uses a stronger background, bold text, and a brighter underline. Syntax and status colors are preserved. With a query, `↑` / `↓` wrap through previous/next matches; the input title shows the current match and total. No matches shows `0/0` and keeps the selection. Clearing the query restores ordinary row navigation.
- The commit graph supports locating loaded commits only, preserving topology; load more to search older commits. The file tree supports background path filtering; focused text contents support locating only. Images and binary notices do not offer locating.
- Settings, overview, help, read-only details and diff content previews offer command input only. They do not search hidden lists or offer search-mode commands. Returning to a list restores its query; keyboard, clicks and wheel focus changes behave consistently. Search, status filters, sorting and file checkboxes remain independent.

## Context prefixes
- `/repo diff`, `/repo log`, `/repo branch` and `/repo stash` open full views. Back restores the previous focus and selection. Pressing `/` in a pane only opens its command context. Settings, schedule and help also restore their origin on return.
- The view prefix is separate and supports segment selection: /repo diff for files and diffs, /repo log for history, /repo branch for branches, and /repos, /settings, /schedule for management pages. Home opens global / commands; overview uses /repo and stash uses /repo stash.
- Type only the operation suffix, e.g. stage, unstage, layout or refresh in a worktree diff. Name and bilingual description matching stay within the current group.
- With an empty suffix, Backspace selects only the last prefix segment, such as `branch` in `/repo branch`. ← expands left by one segment; → shrinks by one, or restores editing when only the last segment remains. Typing or pasting plain text replaces only the selection: replacing `branch` with `log` produces `/repo log`. Pasting a command starting with `/` or `//` uses global scope. Candidates follow the retained parent, or show global commands when everything is selected. Use ↑↓ to select, Tab to complete, or click to run. Only selected segments use the original selection colors.
- Another / with an empty command suffix switches to global commands too; typing //settings from search opens global settings directly.
- Backspace / Delete removes the selection while retaining any parent; deleting the entire prefix closes commands. Enter or Esc cancels selection and restores the original prefix without running a command. With no prefix selected, Esc closes commands and restores search. Editing never navigates or changes the frozen repository and copy target; reopening captures the current focus.
- With existing search, Home then / opens commands while retaining search. Slashes inside a search path remain literal text.
- Historical diffs offer layout options; use global `/copy` for the loaded patch. They do not offer staging or refresh. Directory completion, configuration review and layout-picker cancellation retain context, drafts and cursor positions.

## Copy the current selection
- Global `/copy` copies the current selection. Repository contexts also list `/copy`, `/refresh`, `/terminal` and `/editor` with full names. Type a name or description to filter; Tab switches explicitly to the global command, Enter runs it. Unavailable selections are disabled.
- In file lists, `/copy` copies marked file paths or the cursor file when none are marked. Paths are relative, deduplicated and separated by newlines; Unicode and spaces stay literal.
- In a focused diff preview, `/copy` copies the loaded original patch without UI line numbers, colors or wrapped lines. It never rereads Git.
- `/copy` copies the selected full commit ID in the graph, or the selected file path in commit details. Historical diffs use `/copy`.
- `/copy` copies the selected branch name. In operation errors, `/copy` copies complete error details.
- Combined panes follow focus. Failed previews or panes copy their error details. Opening commands freezes the repository and selection; background refresh cannot replace the target.
- Images, loading, empty and truncated patches report explicit errors. Clipboard writes run independently, report success after the system accepts them and preserve navigation and new drafts.
- Linux needs X11 or a Wayland session supporting data-control. Content retention after exit depends on the desktop clipboard manager. Error pages accept only copy and quit commands.

## Paths and completion
- `Tab` completes the directory component at the cursor. One match adds `/`; multiple matches show directory candidates.
- In candidates, `↑↓` selects; `Tab` / `Enter` fills the selected directory and keeps editing; `Esc` closes the menu. Typing filters the current directory candidates.
- Directory, branch and stash candidates load in the background. Editing paths or switching commands cancels obsolete requests; late results cannot overwrite a newer draft. Loading feedback appears only after a brief delay.
- With precise clicks enabled, clicking a directory candidate fills its path. Continue editing, then press `Enter` to review the operation.
- `~` and `~/…` use your user home, independent of the application's data directory. Review and execution use absolute paths.
- Spaces and Chinese text are literal. Matching single or double quotes may enclose the entire path, including `~/…`.
- Hidden directories appear after a `.` prefix. Completion reads one directory level and retains the path after the cursor.
> A path is the entire remaining argument. No variable, wildcard or shell escape expansion occurs. Named-user paths such as ~user are unsupported.

## Branches, fetching and stashes
- After `branch switch `, local names and tip subjects appear as parameter candidates. Type to filter, use arrows to choose, Tab to fill, and Enter to review. Completion does not inspect remote tracking or ahead/behind counts.
- Stash parameters show indices, short IDs and messages. Filter by ID or message, use Tab to fill the complete object ID, or Enter/click to review. Repository changes still require confirmation.
- `/repo branch create <name>` creates a local branch at the current commit without switching. `/repo branch switch <name>` switches to an existing local branch. Tracked files must be clean; untracked files are protected, and branches checked out in another worktree are refused.
- `/repo fetch` fetches configured remote-tracking branches immediately in the background, honoring refspecs, exclusions and skipFetchAll. It does not change the worktree or push.
- `/repo pull` fetches once and reviews a pinned upstream commit. Confirmation allows fast-forward only; divergence and dirty worktrees are explicit errors. No automatic merge, rebase or stash.
- Synchronization review shows its current stage on the reserved footer row. Task details preserve stage timings separately from failure reasons.
- `/repo sync` refreshes the remote and reviews the current branch's required fast-forward pull or push before confirmation. Already-synchronized branches show a notice. Divergence requires a merge or rebase first; missing remotes or remote upstreams show an availability notice rather than a repository failure. Results appear in background tasks.
- `/repo stash` opens a searchable list. Use ↑↓ to select, Enter for details, and ←/Esc to return. Slash commands use the /repo stash context; search matches messages and object IDs.
- `/repo stash save [tracked|all|selected]` opens scope choices when the value is omitted. Tab completes, Enter chooses; selected shows the checked file count and is disabled when nothing is checked.

Press Enter on a stash to inspect its saved message and files. Wide windows show one continuous diff beside the file list; selecting a file follows its section. Enter opens the full file diff, and Esc preserves selection and search when returning. Narrow windows show the message and files first. /focus changes panes in stash details; `/focus` and `/zoom` use the shared pane controls.

`/repo stash scope [all|staged|unstaged]` selects the saved comparison: `all` compares the stash base with its saved worktree and includes saved untracked files; `staged` shows the original staged changes; `unstaged` compares the saved index with the saved worktree and includes untracked files. All content comes from immutable stash objects. A file staged and then reverted to its base has no final diff under `all`; inspect its two changes under `staged` and `unstaged`.
- `tracked` saves all staged and unstaged tracked edits, including added files; `all` also includes untracked files. `selected` saves only checked workspace files, including their staged/unstaged edits and chosen new files. Renames include both paths; targets are captured when the command opens. Ignored untracked files are excluded in every mode.
- The review shows the scope and actual paths. The message is optional and defaults to Git's generated message. Shift+Tab switches input/options; ←→ selects Save or Cancel; Enter confirms the focused control and Esc returns to the command draft. PgUp/PgDn, the wheel, and scrollbars browse the paths. Clicking a button selects it; double-clicking matches Enter. Cancelling saves nothing.
- `/repo stash apply <stash-id>` restores and retains the entry. `/repo stash pop <stash-id>` removes the entry only after successful restoration. `/repo stash drop <stash-id>` removes it without restoring. Completing these commands from the list fills the selected full object ID for review.
- `/copy` copies the selected full object ID; `/refresh` reloads the list. Restore conflicts are reported as failures and retain the original stash and actual conflict state.
- Writes require review and confirmation; returning to edit preserves the draft. HEAD, index, worktree and target references are revalidated. Stash operations also validate reflog order; external changes require a new review.

## Links and paths
- Click underlined URLs and paths to open them. URLs use the default browser; local files and directories use the system default application. Precise-click preferences apply. Lists, directory toggles and scrollbars keep their existing behavior.
- Markdown labels retain their original destinations. Relative file links resolve against the current document directory, including Unicode, spaces and percent encoding. Scrolling, wrapping and splits do not truncate destinations.
- In text selection mode, native links are handled by the terminal. Ghostty normally uses Command-click on macOS and Ctrl-click on Linux; terminal settings determine the gesture.
- External opens run in the background and report failures. Missing files do not redirect to another location.

## Repository file browser
- Long tree names and paths scroll horizontally. The bottom border shows a scrollbar only for overflow; Git status columns stay fixed. Use a horizontal wheel, Shift + vertical wheel, or click and drag the scrollbar, including while filtering paths. With empty input, left/right arrows scroll horizontally; with text they move the cursor.
- A path breadcrumb appears above the browser; the current file name uses bold white text at the end of the path, without a duplicate content panel title. Click an ancestor to return to its directory in the tree, or the repository name to return to the root. This clears the path filter while retaining expanded directories and per-file content searches and viewports. Narrow windows omit middle components; very short windows prioritize content.
- `/repo files` browses tracked files and untracked files allowed by Git ignore rules. Directories load one level at a time in the background; `.git` is excluded.
- Directory chains with a single child directory and no other visible entries share one `parent/child/` row. Files and forks stop compaction.
- `↑↓` selects; `Enter` toggles a directory or focuses a selected file's preview. Left/right arrows scroll horizontally, /focus switches panes, and Esc returns.
- Single-click a directory row to expand or collapse it; a double-click toggles it only once. Single-click files to select them; double-click to focus their preview. Precise-click preferences apply; scrollbar clicks only seek.
- `/focus` switches between tree and preview; `/zoom` maximizes the focused pane. Narrow windows display the focused pane. Previews support syntax highlighting, images and both scroll axes.
- Typing searches paths throughout the repository in the background, with an explicit 1,000-match limit. `Esc` clears search before returning to the tree or previous view.
- `/repo files diff` opens the selected file's changes and preserves the browser when returning. `/refresh` reloads directories and preview; `/copy` copies the selected path.
- Two fixed columns at the end of each row describe index and worktree changes: `M` modified, `A` added, `D` deleted, `R` renamed, `T` type changed, `??` untracked, `UU` conflicted.
- Markdown and SVG open as source by default. Use the command menu to choose source, rendered preview or split. Ordinary files do not offer these commands.
- `/repo files mode [source|preview]` selects source or rendered preview for the current file. An empty parameter opens one option menu with ✓ on the current value; Tab fills, Enter applies, and Esc cancels. The active mode is muted with a reason. `/repo files layout [single|split]` selects a single pane or split through the same option menu. Choosing the current layout leaves it unchanged. Choosing source or preview from a split opens the corresponding single pane. Each file retains its presentation and separate source/preview scroll positions for the current session.
- In a split, `/focus` switches through the tree, source and preview. Pointer focus and scrolling respect the pointer settings. Narrow windows stack the panes; the smallest windows show the focused pane while retaining the split for later resizing.
- SVG uses resvg in the background at the viewport size and supports embedded resources; external image references report an error. Markdown reuses the help page renderer; images retain their descriptions and addresses.

- Type in focused file contents to locate text; ↑↓ cycles every occurrence and the title shows the current match/total. Matching ignores case, supports Unicode and excludes line numbers. Highlighting keeps syntax colors and links; long lines, wrapping and resizing reveal the current match. Esc clears the query before returning to the tree. Tree filtering, source locating and Markdown-preview locating retain independent drafts; split views search the focused side.

## Files, commits and pushes
- `/repo diff` opens the worktree. Click a tab above the file list or run `/repo diff unstaged` or `/repo diff staged` to switch views. Each view remembers its cursor, scroll position and checked files. The preview shows the current file.
- Wide windows show files beside the preview; narrow windows stack them. `/focus` switches focus; `Enter` / `→` focuses the preview, and `/zoom` maximizes the focused pane.
- `Esc` first clears input, then restores a maximized pane, then returns focus to files. Another `Esc` returns to the repository pane or list.
- Diff defaults to side-by-side on wide screens and unified on narrow screens. Long lines stay on one row; both sides scroll together with fixed line numbers.
- `/repo diff context [full|changes]` chooses full source or changes only. Full source is the default; the preference survives restart independently of split/unified layout. Staging preserves the reading position while the file stays in the current view. A disappearing selection is cleared; use arrows or click another change to continue. Copying still uses the original patch.
- `/repo diff header [show|hide]` selects whether original Git file headers are visible. Omitting the value opens an option menu; choosing the current value leaves it unchanged. It starts collapsed; the choice applies across files and views and survives restart. Collapsing restores the previous reading position. Paths and object IDs are hidden from the patch body; file-mode changes and similarity appear in the information area. Hunk headings and binary notices remain; a missing final newline has a separate red `⊖` marker below the affected last line, without a line number or line selection. Split view marks each affected side independently. Copying preserves the full original patch.
- Confirming `/repo diff layout` or entering its trailing space opens auto/split/unified parameter candidates immediately. Use ↑↓ to select, Tab to fill, and Enter to apply. Esc closes parameter candidates and preserves the draft; another Esc returns to the view.
- Command candidates show the current setting; Auto also identifies the visible unified or split layout and updates on resize. Click a command row or press `Enter` to open it.
- `/repo diff layout auto`, `/repo diff layout split`, `/repo diff layout unified` change layout immediately and save the preference for future launches. These are parameter values; a space opens choices and Tab completes the value.
- Click a directory or press Enter to fold/unfold it. Click a checkbox to toggle selection; click a file name to focus it, or double-click to enter. This follows the precise-click preference. `Space` marks with empty input; `Insert` always marks; `Ctrl+A` marks all visible files in the current view. Filtering retains hidden selections. A directory checkbox applies to all matching descendants in the same staged or unstaged group, including folded files; `✓` means all checked and `−` means partially checked.
- In the file list, `/repo diff stage` stages files in the unstaged view, and `/repo diff unstage` unstages files in the staged view. Both use the current view’s checked files, or the focused file or directory’s matching files when nothing is checked. `/repo diff discard-file` reviews whole-file discard in the unstaged view.
- The focused preview defaults to hunk selection; ↑↓ changes hunks. `/repo diff select [hunk|line]` changes granularity. In line mode, Shift+↑↓ extends or shrinks the changed-line range; clicking a changed line locates it. Unified, split and wrapped layouts share the selection. PgUp/PgDn and the wheel only scroll the preview.
- Selected changes have blue line-number gutters and a blue thin `│` separator. Code retains its added/removed backgrounds and syntax colors. Split columns mark their own selections, including wrapped continuations. Apply the selection with stage, unstage or discard.
- In the preview, Space with empty input stages selected unstaged changes or unstages selected staged changes. The stage/unstage commands also apply only to this selection. `/repo diff discard` reviews selected unstaged changes with Return selected by default; confirm to discard.
- Partial operations support regular text, including new and deleted files. Renames, conflicts, symlinks, submodules, binaries, truncated diffs and external content filters are unsupported. Expand a selection that cannot apply independently; refresh stale file or index snapshots. Historical diffs stay read-only.
- Stage, unstage and discard refresh both lists and keep the current view and any new command draft.
- Unstaging retains worktree contents. Git-ignored files are not listed.
- `/repo commit` shows staged changes above a multiline message editor. `Enter` inserts a newline and `↑↓` moves the cursor. `Shift+Tab` switches between the editor and horizontal options; on the action bar, `←→` selects, `Enter` runs the selection and `↑` returns to editing. Selecting Commit executes it without another preview; `Ctrl+S` also commits, and `PgUp/PgDn` reviews staged contents. Git hooks and signing are preserved.
- `/repo push` validates and performs ordinary pushes directly. If the remote contains commits absent locally, a separate confirmation (cancel selected by default) allows an exact `--force-with-lease` push. Without an upstream, select a remote and edit the target branch, then press Enter to publish and save tracking on success. Git’s `push.autoSetupRemote=true` publishes the same-name branch directly when its default destination is unambiguous. Scheduled workflows remain non-forced.
- `/terminal` opens the system default terminal in the selected repository; type `/te` then `Tab` to complete.

## Image previews
- Selecting a PNG, JPEG or WebP in `/repo diff` automatically previews both versions; in commit details, press `Enter` to compare. Content detection does not require an extension.
- Wide windows show versions side by side; narrow windows with sufficient height stack them. Short panes retain side-by-side images. Images keep their aspect ratio and use a checkerboard for transparency.
- `Esc` returns; resizing fits images to the pane. Missing versions, corrupt images and unavailable pixel support are explicit.
- Index and historical images come from Git objects, never from current worktree contents.

## Commit graph
- `/repo log` reads local branches, remote-tracking branches, tags and HEAD without fetching.
- `↑↓` / `PgUp` / `PgDn` selects; `Enter` opens commit details and then a historical file diff; empty-input `Esc` goes back.
- Each page contains up to 100 commits; `↓` at the bottom or `/repo log more` loads the next page. `Ctrl+R` refreshes the reference snapshot.
- Type to locate loaded commits by hash, title, author or ref. Nodes remain visible; `↑` / `↓` wraps through previous / next matches.
- With an empty input, ←→ scrolls overflowing graph lanes or text while short hashes stay fixed. Wide terminals show a metadata preview.
- Commit details show the message and metadata above the file list. Wide screens add one continuous unified diff on the right. Selecting a file locates its diff; Enter opens the full file diff, and Back preserves selection and position. Narrow screens hide the continuous preview.
- Type to filter or locate in the file pane; /repo log parent switches merge parents. The wheel scrolls each pane independently. Use /focus to switch panes and /zoom to maximize or restore. In the message or preview pane, arrows and PgUp/PgDn scroll content.
- Root commits compare with the empty tree; other diffs identify the parent and selected commit. `/repo diff` layout commands also apply.
> Graph and historical diff reads do not stage, commit, fetch, push or change reports.

## Fetch and configuration
- Experimental features are enabled by default and remembered in settings. AI commit composition uses a distinct text color for its commands and page title; disabling experiments hides the whole composition group while keeping ordinary commit-message generation available.
- `/settings auto-fetch [true|false]` enables periodic fetch in this TUI (off by default). `/settings auto-fetch-interval <duration>` sets its interval (3m by default, at least 60s). It waits while unfocused or busy and fetches once when due, without catch-up runs. Startup and manual fetches reset the deadline; background completion preserves input and navigation. System scheduling remains independent.
- Startup reads local state by default. `/settings startup-fetch [true|false]` enables one background fetch on the next launch for managed repositories and the current session repository, using the same operation as manual fetch. It does not run scheduled pull or push steps.
- `/repo fetch` fetches the current repository and refreshes its status; `/repos fetch` fetches all managed repositories.
- Local automatic refresh coalesces filesystem events for 250 ms, or up to 1 second during continuous changes. Terminal focus loss pauses automatic reads while changes accumulate; focus gain refreshes affected repositories. Manual operations, startup fetch and scheduled tasks continue running.
- Checks run in the background. Browse, search and view files while they run. A spinner marks active checks; unfinished rows retain previous results.
- `/cancel` cancels the selected repository task, or the batch check when no selected task exists. Independent repositories run concurrently; operations sharing Git storage serialize. Configuration writes use a short transaction.
- `/repo` opens the selected repository; `/repos` returns to the list. `/repos manage` manages registrations; `/repos add <path>` adds one repository and `/repos remove` unregisters the selection.
- `/repos scan <path> [--exclude <path>…]` scans and opens a selection review. Add selected registers checked repositories. Already added repositories are marked; unreadable entries show their error. Space/Insert toggles, Ctrl+A selects matching new repositories, Shift+Tab switches input/options, Enter activates and Esc returns.
- One `--exclude` accepts multiple directories: `/repos scan ~/Proj --exclude backups archive "old projects"`. Relative exclusions resolve against the scan root, and each path supports Tab completion. Exclusions and unchecked entries affect only this scan.
- `/settings concurrency <count>` sets bounded concurrency, from 1 to 32.
- In a workflow editor, check a step to include it or uncheck it to remove it. `↑↓` selects steps and the Save/Cancel buttons; `Enter`/Space activates the selection. `←→` selects reordering controls in the separate Actions panel. Clicking a step also toggles it. Save applies the draft.
- `/settings wrap [true|false]` saves text wrapping for file previews and diffs (off by default). Gutters stay fixed, split diffs remain aligned, and links keep their complete targets. When off, use horizontal scrolling. The same option is in Settings → Display.
- `/schedule` shows status, interval, time basis, and workflow together with their associated buttons. Click a button to edit or enter enable/disable confirmation; use ↑↓ and Enter with the keyboard.
- `/schedule actions` edits scheduled actions; `/schedule interval <duration>` accepts values such as 6h.
- `/schedule anchor <datetime|auto>` sets a local time basis (HH:MM[:SS] or YYYY-MM-DD HH:MM[:SS]); auto regenerates it on enable.
- The first enable persists an automatic basis when omitted. Restarts and re-enabling preserve it; the schedule page shows the basis and applied next trigger.
- Durations support s / m / h / d / w (at least 60 seconds). Apply saved changes with `/schedule apply`.
- `/schedule apply` applies configuration; `/schedule enabled [true|false]` selects whether scheduling is enabled. Omitting the value opens an option menu with the current state marked; execution requires confirmation.
> The scheduled workflow defaults to fetch → inspect. Add and reorder pull or push explicitly. Inspect is last; push requires an earlier fetch or pull. Failure stops later steps for that repository; task details retain step outcomes and times. Manual checks still fetch and read local status only.

## Confirmation
- `←→` selects an action; `Enter` confirms; `Esc` returns. Returning from commit or configuration confirmation retains the draft.
- Commit messages use the bottom input. Repository reads and writes run in the background; navigation and command input remain available. Completion updates its repository and does not replace another view or a new command draft.
- `/cancel` cancels repository work; `Esc` returns through views. Confirming `Ctrl+C` exit or executing `/quit` exits, cancelling and collecting owned tasks and Git processes. During blocking operations such as configuration writes, `Esc` / `Ctrl+C` only requests cancellation and waits for child processes; it does not arm exit confirmation.

## Status legend
- Green `✓` Normal     Yellow `!` Needs attention     Red `×` Failed     Gray `·` Unchecked; a spinner means running. Header counts match row markers. Failures take priority and each repository belongs to one category; details preserve successful and failed branch results.
- `*` Worktree changes: uncommitted, untracked or submodule changes. Worktree changes or nonzero sync counts on the checked-out branch need attention and appear in yellow. Other branches do not affect the home category; repository names keep their normal color. `?` means unknown.
- The list merges per-repository versions from checks and local operations. Newer repository results win over late batch results. The file panel reads current local state.
- The home detail panel shows the current branch above its sync counts, using the same snapshot, including repositories without remotes. Unborn branches keep their symbolic branch name; detached HEAD shows Detached HEAD, and unavailable snapshots show ?. In narrow windows, open the repository overview to see its branch.
- No-upstream branches are counted separately. They are not problems and are never published automatically.
- Home sync shows ↑ahead and ↓behind commit counts for the checked-out branch against its upstream. Other branches remain in repository details and the branch panel. — means no upstream, a missing upstream ref, detached HEAD, or no commits yet; these states alone do not need attention. ? means the saved record lacks current-branch data; run another check to refresh it. The problems filter includes attention and failures; failed includes failures only. Push outcomes remain in task history and details.
- Full errors appear in preview and repository details.

## Scrolling indicators
- Scrollbars support clicking the track and dragging the thumb on both axes. Dragging stays with the original pane until release, including outside the track; the precise-click preference applies.
- Vertical scrollbars sit in the right border over content rows only, excluding fixed headers. Unstaged and staged groups have separate tracks; content that fits has no scrollbar.
- At the repository root, arrows select panes. Inside focused content, use global `/focus` to switch panes; arrows retain their content actions.
- In a diff preview with empty input, ←→ scrolls long lines. Line numbers and change markers stay fixed; split columns pan together. Esc goes back. Overflowing content has a scrollbar embedded in the bottom border without taking a content row.
- In history, ←→ scrolls overflowing graph lanes, refs and subjects together; each region stops at its own boundary and short hashes stay fixed. Enter opens details, and Esc goes back.
- Horizontal wheel and Shift + vertical wheel also work when forwarded by the terminal. With text in the input, arrow keys edit the input; command and confirmation selectors keep their own keys.

<!-- optional-ai -->

## Language
- System Chinese uses Chinese; all other languages use English.
- Set `APP_LANG=en` or `APP_LANG=zh` before starting to override the system language.
