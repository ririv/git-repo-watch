
## Developer feature: system scheduler
- In a workflow editor, check a step to include it or uncheck it to remove it. `↑↓` selects steps and the Save/Cancel buttons; `Enter`/Space activates the selection. `←→` selects reordering controls in the separate Actions panel. Clicking a step also toggles it. Save applies the draft.
- `/schedule` shows status, interval, time basis, and workflow together with their associated buttons. Click a button to edit or enter enable/disable confirmation; use ↑↓ and Enter with the keyboard.
- `/schedule actions` edits scheduled actions; `/schedule interval <duration>` accepts values such as 6h.
- `/schedule anchor <datetime|auto>` sets a local time basis (HH:MM[:SS] or YYYY-MM-DD HH:MM[:SS]); auto regenerates it on enable.
- The first enable persists an automatic basis when omitted. Restarts and re-enabling preserve it; the schedule page shows the basis and applied next trigger.
- Durations support s / m / h / d / w (at least 60 seconds). Apply saved changes with `/schedule apply`.
- `/schedule apply` applies configuration; `/schedule enabled [true|false]` selects whether scheduling is enabled. Omitting the value opens an option menu with the current state marked; execution requires confirmation.
> The scheduled workflow defaults to fetch → inspect. Add and reorder pull or push explicitly. Inspect is last; push requires an earlier fetch or pull. Failure stops later steps for that repository; task details retain step outcomes and times. Manual checks still fetch and read local status only.
