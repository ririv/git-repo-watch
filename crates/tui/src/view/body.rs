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
        Mode::Branches(view) => branches::render(frame, area, view),
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
        Mode::Files(s) => files::render(
            frame,
            area,
            s,
            &format!("{} · {}", name(&s.snapshot.path), safe(s.snapshot.branch())),
        ),
        Mode::Diff {
            content, scroll, ..
        } => document::render(
            frame,
            area,
            content,
            scroll,
            screen.diff_preference,
            &mut screen.graphics,
        ),
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
            Page::Detail => overview::render(frame, screen, area),
            Page::Configuration(Configuration::Schedule) => schedule::render(frame, screen, area),
            Page::Configuration(section) => settings(frame, screen, area, section),
            Page::Help => help::render(frame, area, &mut screen.scroll),
        },
    }
}
