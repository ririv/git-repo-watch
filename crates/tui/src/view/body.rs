use super::*;
pub(super) fn render_body(frame: &mut Frame, screen: &mut Screen, area: Rect) {
    if screen.page == Page::Detail
        && screen.workspace.path().is_some()
        && crate::workspace::Workspace::is_root(&screen.mode)
    {
        workspace::render(frame, screen, area);
        return;
    }
    match &mut screen.mode {
        Mode::Browser(session) => browser::render(
            frame,
            area,
            session,
            &mut screen.graphics,
            &mut screen.pointer,
        ),
        Mode::Stashes(view) => stashes::render(frame, area, view, &mut screen.pointer),
        Mode::Branches(view) => branches::render(frame, area, view, &mut screen.pointer),
        Mode::History(view) => history::render(
            frame,
            area,
            view,
            screen.diff_preference,
            &mut screen.graphics,
            &mut screen.pointer,
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
                &mut screen.pointer,
            );
        }
        Mode::Files(s) => {
            let title = format!("{} · {}", name(&s.snapshot.path), safe(s.snapshot.branch()));
            files::render(
                frame,
                area,
                s,
                &title,
                screen.file_layout,
                &mut screen.pointer,
            );
        }
        Mode::Changes(session) => changes::render(
            frame,
            area,
            session,
            screen.diff_preference,
            screen.file_layout,
            &mut screen.graphics,
            &mut screen.pointer,
        ),
        Mode::CommitMessage { plan, scroll, .. } => text_view(
            frame,
            area,
            &commit_review(plan),
            scroll,
            i18n::tr!(CommitPreview),
            &mut screen.pointer,
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
            &mut screen.pointer,
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
            &mut screen.pointer,
        ),
        Mode::Error { detail, scroll, .. } => text_view(
            frame,
            area,
            &multiline(detail),
            scroll,
            i18n::tr!(OperationResult),
            &mut screen.pointer,
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
                Confirmation::Mutation { plan, .. } => multiline(&crate::mutations::review(plan)),
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
            Page::View(view) => {
                frame.render_widget(Paragraph::new(view.title()), area);
            }
            Page::Repositories => repositories(frame, screen, area),
            Page::Detail => overview::render(frame, screen, area),
            Page::Configuration(Configuration::Schedule) => schedule::render(frame, screen, area),
            Page::Configuration(section) => settings(frame, screen, area, section),
            Page::Help => help::render(frame, area, &mut screen.scroll, &mut screen.pointer),
        },
    }
}
