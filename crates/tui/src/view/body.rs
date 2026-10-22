use super::*;
pub(super) fn render_body(frame: &mut Frame, screen: &mut Screen, area: Rect) {
    match &mut screen.mode {
        Mode::History(view) => history::render(
            frame,
            area,
            view,
            screen.diff_preference,
            &mut screen.graphics,
        ),
        Mode::Busy(pending) if matches!(pending.origin.as_ref(), Mode::History(_)) => {
            let Mode::History(view) = pending.origin.as_mut() else {
                unreachable!()
            };
            history::render(
                frame,
                area,
                view,
                screen.diff_preference,
                &mut screen.graphics,
            );
        }
        Mode::Files(s) => {
            let title = format!("{} · {}", name(&s.snapshot.path), safe(s.snapshot.branch()));
            let frame_block = block(title);
            let inner = frame_block.inner(area);
            frame.render_widget(frame_block, area);
            let parts = Layout::vertical([Constraint::Length(2), Constraint::Min(1)]).split(inner);
            let rows = s.rows();
            frame.render_widget(
                Paragraph::new(i18n::tf!(
                    FilesSelected,
                    if s.area == FileArea::Index {
                        i18n::tr!(Index)
                    } else {
                        i18n::tr!(Worktree)
                    },
                    rows.len(),
                    s.selected.len()
                )),
                parts[0],
            );
            let items = rows
                .iter()
                .map(|c| {
                    ListItem::new(format!(
                        "[{}] {} {}",
                        if s.selected.contains(&s.change_index(c)) {
                            "✓"
                        } else {
                            " "
                        },
                        c.kind,
                        safe(&c.location.display())
                    ))
                })
                .collect::<Vec<_>>();
            let mut state = ListState::default().with_selected(if items.is_empty() {
                None
            } else {
                Some(s.cursor)
            });
            frame.render_stateful_widget(
                List::new(if items.is_empty() {
                    vec![ListItem::new(i18n::tr!(NoChangesInThisArea))]
                } else {
                    items
                })
                .highlight_style(selected_style())
                .highlight_symbol("› "),
                parts[1],
                &mut state,
            );
        }
        Mode::Diff {
            content, scroll, ..
        } => diff::render(frame, area, content, scroll, screen.diff_preference),
        Mode::CommitMessage { plan, scroll, .. } => text_view(
            frame,
            area,
            &commit_review(plan),
            scroll,
            i18n::tr!(CommitPreview),
        ),
        Mode::CommitConfirm {
            plan,
            message,
            scroll,
            ..
        } => text_view(
            frame,
            area,
            &i18n::tf!(CommitMessage, safe(message), commit_review(plan)),
            scroll,
            i18n::tr!(ConfirmCommit),
        ),
        Mode::PushConfirm { plan, scroll, .. } => text_view(
            frame,
            area,
            &i18n::tf!(
                RepositoryBranchRemoteUpstreamUrlPendingCommits,
                safe(&plan.path.to_string_lossy()),
                safe(&plan.target.branch),
                safe(&plan.target.remote),
                safe(&plan.target.remote_ref),
                safe(&plan.url),
                plan.target.ahead_commits
            ),
            scroll,
            i18n::tr!(ConfirmPush),
        ),
        Mode::Error { detail, scroll, .. } => text_view(
            frame,
            area,
            &multiline(detail),
            scroll,
            i18n::tr!(OperationResult),
        ),
        Mode::Confirm {
            operation: confirmation,
            ..
        } => {
            let text = match confirmation {
                Confirmation::Schedule(operation) => match operation {
                    engine::scheduler::Control::Enable => i18n::tr!(ScheduleEnableConfirmation),
                    engine::scheduler::Control::Disable => i18n::tr!(ScheduleDisableConfirmation),
                    engine::scheduler::Control::Apply => i18n::tr!(ScheduleApplyConfirmation),
                }
                .into(),
                Confirmation::Remove(path) => i18n::tf!(
                    UnregisterRepositoryFilesOnDiskAreRetained,
                    safe(&path.to_string_lossy())
                ),
                Confirmation::Configure { command, value, .. } => {
                    format!(
                        "/{} {}\n\n{}",
                        command.command().name,
                        safe(value),
                        command::description(command.action())
                    )
                }
            };
            frame.render_widget(
                Paragraph::new(text)
                    .wrap(Wrap { trim: false })
                    .block(block(i18n::tr!(ConfirmOperation))),
                area,
            );
        }
        _ => match screen.page {
            Page::Repositories => repositories(frame, screen, area),
            Page::Detail => {
                let rows = screen.rows();
                let text = rows
                    .get(screen.selected())
                    .map(|row| repository_detail(screen, row))
                    .unwrap_or_else(|| i18n::tr!(CurrentRepositoryIsNotInTheFiltered).into());
                review::styled_text_view(
                    frame,
                    area,
                    text,
                    &mut screen.scroll,
                    i18n::tr!(RepositoryDetails),
                )
            }
            Page::Configuration(Configuration::Schedule) => schedule::render(frame, screen, area),
            Page::Configuration(section) => settings(frame, screen, area, section),
            Page::Help => help::render(frame, area, &mut screen.scroll),
        },
    }
}
