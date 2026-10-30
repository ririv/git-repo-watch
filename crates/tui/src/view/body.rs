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
        Mode::Busy(pending) if matches!(pending.origin.as_ref(), Mode::Management(_)) => {
            let Mode::Management(session) = pending.origin.as_mut() else {
                unreachable!()
            };
            management::render(frame, area, session, &mut screen.pointer);
        }
        Mode::Management(session) => management::render(frame, area, session, &mut screen.pointer),
        Mode::Workflow(session) => workflow::render(frame, area, session, &mut screen.pointer),
        Mode::CompositionWaiting(waiting) => {
            composition::waiting(frame, area, waiting, &mut screen.pointer)
        }
        Mode::Composition(session) => {
            composition::render(frame, area, session, &mut screen.pointer)
        }
        Mode::PromptEditor(session) => prompt::render(frame, area, session, &mut screen.pointer),
        Mode::Busy(pending) if matches!(pending.origin.as_ref(), Mode::PromptEditor(_)) => {
            let Mode::PromptEditor(session) = pending.origin.as_mut() else {
                unreachable!()
            };
            prompt::render(frame, area, session, &mut screen.pointer);
        }
        Mode::Browser(session) => browser::render(
            frame,
            area,
            session,
            screen.config.text_wrap,
            screen.config.file_icons.into(),
            &mut screen.graphics,
            &mut screen.pointer,
        ),
        Mode::Stashes(view) => stashes::render(frame, area, view, &mut screen.pointer),
        Mode::Branches(view) => branches::render(frame, area, view, &mut screen.pointer),
        Mode::History(view) => history::render(
            frame,
            area,
            view,
            crate::diff::Options {
                layout: screen.diff_preference,
                wrap: screen.config.text_wrap,
            },
            screen.config.file_icons.into(),
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
                crate::diff::Options {
                    layout: screen.diff_preference,
                    wrap: screen.config.text_wrap,
                },
                screen.config.file_icons.into(),
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
                crate::files::Options {
                    layout: screen.file_layout,
                    icons: screen.config.file_icons.into(),
                },
                &mut screen.pointer,
            );
        }
        Mode::Changes(session) => changes::render(
            frame,
            area,
            session,
            crate::diff::Options {
                layout: screen.diff_preference,
                wrap: screen.config.text_wrap,
            },
            crate::files::Options {
                layout: screen.file_layout,
                icons: screen.config.file_icons.into(),
            },
            &mut screen.graphics,
            &mut screen.pointer,
        ),
        Mode::CommitMessage {
            plan,
            editor,
            scroll,
        } => commit::render(
            frame,
            area,
            plan,
            editor,
            scroll,
            &mut screen.pointer,
            screen.bar.is_plain(),
        ),
        Mode::PushTarget(session) => super::push::target(frame, area, session, &mut screen.pointer),
        Mode::PushConfirm { plan, scroll, .. } => text_view(
            frame,
            area,
            &multiline(&crate::push::review(plan)),
            scroll,
            if plan.force_behind().is_some() {
                i18n::tr!(PushForceTitle)
            } else {
                i18n::tr!(PushRecreateTitle)
            },
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
                Confirmation::Remove { path, .. } => i18n::tf!(
                    UnregisterRepositoryFilesOnDiskAreRetained,
                    safe(&path.to_string_lossy())
                ),
                Confirmation::ReplayParents {
                    candidate,
                    selected,
                    ..
                } => {
                    super::replay::parents(frame, area, candidate, *selected, &mut screen.pointer);
                    return;
                }
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
            Page::Help => help::render(
                frame,
                area,
                &mut screen.scroll,
                &mut screen.pointer,
                screen.config.ai.enabled,
            ),
        },
    }
}
