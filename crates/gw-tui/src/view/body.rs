use super::*;
pub(super) fn render_body(frame: &mut Frame, screen: &mut Screen, area: Rect) {
    match &mut screen.mode {
        Mode::Files(s) => {
            let title = format!("{} · {}", name(&s.snapshot.path), safe(s.snapshot.branch()));
            let frame_block = block(title);
            let inner = frame_block.inner(area);
            frame.render_widget(frame_block, area);
            let parts = Layout::vertical([Constraint::Length(2), Constraint::Min(1)]).split(inner);
            let rows = s.rows();
            frame.render_widget(
                Paragraph::new(gw_i18n::tf!(
                    FilesSelected,
                    if s.area == FileArea::Index {
                        gw_i18n::tr!(Index)
                    } else {
                        gw_i18n::tr!(Worktree)
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
                    vec![ListItem::new(gw_i18n::tr!(NoChangesInThisArea))]
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
        } => text_view(
            frame,
            area,
            &multiline(content),
            scroll,
            gw_i18n::tr!(FileDiff),
        ),
        Mode::CommitMessage { plan, scroll, .. } => text_view(
            frame,
            area,
            &commit_review(plan),
            scroll,
            gw_i18n::tr!(CommitPreview),
        ),
        Mode::CommitConfirm {
            plan,
            message,
            scroll,
        } => text_view(
            frame,
            area,
            &gw_i18n::tf!(CommitMessage, safe(message), commit_review(plan)),
            scroll,
            gw_i18n::tr!(ConfirmCommit),
        ),
        Mode::PushConfirm { plan, scroll } => text_view(
            frame,
            area,
            &gw_i18n::tf!(
                RepositoryBranchRemoteUpstreamUrlPendingCommits,
                safe(&plan.path.to_string_lossy()),
                safe(&plan.target.branch),
                safe(&plan.target.remote),
                safe(&plan.target.remote_ref),
                safe(&plan.url),
                plan.target.ahead_commits
            ),
            scroll,
            gw_i18n::tr!(ConfirmPush),
        ),
        Mode::Error { detail, scroll, .. } => text_view(
            frame,
            area,
            &multiline(detail),
            scroll,
            gw_i18n::tr!(OperationResult),
        ),
        Mode::Input { action, .. } => frame.render_widget(
            Paragraph::new(command::description(*action))
                .wrap(Wrap { trim: false })
                .block(block(gw_i18n::tr!(ConfigurationInput))),
            area,
        ),
        Mode::Confirm(confirmation) => {
            let text = match confirmation {
                Confirmation::CheckAll => gw_i18n::tf!(
                    CheckAllRegisteredRepositoriesConcurrencyLimitAutomatic,
                    screen.config.repositories.len(),
                    screen.config.max_concurrency,
                    screen.config.push_policy
                ),
                Confirmation::Apply => gw_i18n::tr!(ApplySavedIntervalAndReloadTheSystem).into(),
                Confirmation::Remove(path) => gw_i18n::tf!(
                    UnregisterRepositoryFilesOnDiskAreRetained,
                    safe(&path.to_string_lossy())
                ),
                Confirmation::Configure { action, value } => {
                    format!("{}\n\n{}", command::description(*action), safe(value))
                }
            };
            frame.render_widget(
                Paragraph::new(text)
                    .wrap(Wrap { trim: false })
                    .block(block(gw_i18n::tr!(ConfirmOperation))),
                area,
            );
        }
        _ => match screen.page {
            Page::Repositories => repositories(frame, screen, area),
            Page::Detail => {
                let rows = screen.rows();
                let text = rows
                    .get(screen.selected)
                    .map(detail_text)
                    .unwrap_or_else(|| gw_i18n::tr!(CurrentRepositoryIsNotInTheFiltered).into());
                review::styled_text_view(
                    frame,
                    area,
                    text,
                    &mut screen.scroll,
                    gw_i18n::tr!(RepositoryDetails),
                )
            }
            Page::Settings => settings(frame, screen, area),
            Page::Help => text_view(
                frame,
                area,
                HELP.text(),
                &mut screen.scroll,
                gw_i18n::tr!(Instructions),
            ),
        },
    }
}
