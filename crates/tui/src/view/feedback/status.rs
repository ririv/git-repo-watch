//! One primary notice owns the status line; task counts use only spare space.
use super::*;

enum Notice {
    Message(crate::message::Message),
    Running(String),
    Cancelling(String),
    Failure(String),
}
impl Notice {
    fn span(self) -> Span<'static> {
        let (text, color) = match self {
            Self::Message(message) => {
                let color = if message.is_error() {
                    BAD
                } else {
                    Color::Reset
                };
                (message.as_str().to_owned(), color)
            }
            Self::Running(text) => (text, ACCENT),
            Self::Cancelling(text) => (text, WARN),
            Self::Failure(text) => (text, BAD),
        };
        Span::styled(
            safe(text.lines().next().unwrap_or("").trim()),
            Style::default().fg(color),
        )
    }
}

fn repository_activity(screen: &Screen) -> Option<Notice> {
    // 生成计划的等待页已显示完整进度，不重复展示同一项任务。
    if matches!(screen.mode, Mode::CompositionWaiting(_)) {
        return None;
    }
    if let Some((path, task)) = screen
        .scope_path()
        .and_then(|path| screen.repository_tasks.get_key_value(&path))
    {
        let text = format!(
            "{} {} · {}",
            task.animation.symbol(),
            name(path),
            task.label()
        );
        return Some(if task.cancel.is_cancelled() {
            Notice::Cancelling(format!("{} · {}", text, i18n::tr!(Cancelling)))
        } else {
            Notice::Running(text)
        });
    }
    screen.repository_tasks.values().next().map(|task| {
        Notice::Running(i18n::tf!(
            RepositoryTasksRunning,
            task.animation.symbol(),
            screen.repository_tasks.len()
        ))
    })
}

fn primary_notice(screen: &Screen) -> Option<Notice> {
    if !screen.message.is_empty() {
        return Some(Notice::Message(screen.message.clone()));
    }
    if let Mode::Error { title, .. } = &screen.mode {
        return Some(Notice::Failure(format!(
            "{}: {title}",
            i18n::tr!(OperationIncomplete)
        )));
    }
    if let Mode::Busy(pending) = &screen.mode {
        match pending.state {
            TaskState::Cancelling => {
                return Some(Notice::Cancelling(format!(
                    "{} {}: {}",
                    pending.animation.symbol(),
                    i18n::tr!(Cancelling),
                    pending.title
                )));
            }
            TaskState::Running if pending.feedback.visible() => {
                return Some(Notice::Running(format!(
                    "{} {}: {}",
                    pending.animation.symbol(),
                    i18n::tr!(RunningAlt),
                    pending.title
                )));
            }
            TaskState::Running => {}
        }
    }
    if matches!(screen.mode, Mode::CommitMessage { .. })
        && let Some(label) = screen.commit_generator.label()
    {
        return Some(Notice::Running(label));
    }
    if let Some(notice) = repository_activity(screen) {
        return Some(notice);
    }
    if let Some(task) = &screen.check_task {
        return Some(match task.state() {
            TaskState::Running => Notice::Running(task.summary()),
            TaskState::Cancelling => Notice::Cancelling(task.summary()),
        });
    }
    if matches!(screen.snapshot.activity, Activity::Running) {
        return Some(Notice::Running(i18n::tr!(CheckingAltAlt).into()));
    }
    if let Some(message) = screen.tasks.recent_notice() {
        return Some(Notice::Message(message.clone()));
    }
    if let Some(error) = &screen.tasks.error {
        return Some(Notice::Failure(format!(
            "{}: {error}",
            i18n::tr!(TasksStorageError)
        )));
    }
    screen.live.error().map(Notice::Failure)
}

fn fit(line: &Line<'_>, width: usize) -> Line<'static> {
    if line.width() <= width {
        return crate::horizontal::clip(line, 0, width);
    }
    let mut result = crate::horizontal::clip(line, 0, width.saturating_sub(1));
    if width > 0 {
        result
            .spans
            .push(Span::styled("…", Style::default().fg(MUTED)));
    }
    result
}

pub(super) fn line(screen: &Screen, width: usize) -> Line<'static> {
    if let Some(notice) = screen.exit_sequence.notice() {
        return fit(&Line::styled(notice, Style::default().fg(WARN)), width);
    }
    let pending = screen
        .tasks
        .records
        .iter()
        .filter(|record| record.pending())
        .count();
    let running = screen
        .tasks
        .records
        .iter()
        .filter(|record| record.active())
        .count();
    let tail = super::super::tasks::summary::footer(pending, running);
    let Some(notice) = primary_notice(screen) else {
        if pending + running == 0 {
            return Line::default();
        }
        let mut line = Line::default();
        if running > 0 {
            line.spans.push(Span::styled(
                format!("{} ", screen.tasks.animation.symbol()),
                Style::default().fg(ACCENT),
            ));
        }
        line.spans.extend(tail.spans);
        return fit(&line, width);
    };
    let mut main = Line::from(notice.span());
    // 只有完整提示与统计都能放下时才显示统计，绝不为计数截短当前提示。
    if main.width() + tail.width() + 3 <= width {
        main.spans
            .push(Span::styled(" · ", Style::default().fg(MUTED)));
        main.spans.extend(tail.spans);
        return main;
    }
    let link = super::super::tasks::summary::entry();
    if main.width() + link.width() + 3 <= width {
        main.spans
            .push(Span::styled(" · ", Style::default().fg(MUTED)));
        main.spans.extend(link.spans);
        return main;
    }
    fit(&main, width)
}
