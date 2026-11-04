//! One bounded status line combines runtime progress with actionable task history.
use super::*;

enum Notice {
    Running(String),
    Cancelling(String),
    Failure(String),
}
impl Notice {
    fn span(self) -> Span<'static> {
        let (text, color) = match self {
            Self::Running(text) => (text, ACCENT),
            Self::Cancelling(text) => (text, WARN),
            Self::Failure(text) => (text, BAD),
        };
        Span::styled(safe(text.trim()), Style::default().fg(color))
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

fn activities(screen: &Screen) -> Vec<Notice> {
    if let Some(error) = &screen.tasks.error {
        return vec![Notice::Failure(format!(
            "{}: {error}",
            i18n::tr!(TasksStorageError)
        ))];
    }
    if let Mode::Error { title, .. } = &screen.mode {
        return vec![Notice::Failure(format!(
            "{}: {title}",
            i18n::tr!(OperationIncomplete)
        ))];
    }
    let mut notices = Vec::new();
    if let Some(error) = screen.live.error() {
        notices.push(Notice::Failure(error));
    }
    if let Mode::Busy(pending) = &screen.mode {
        match pending.state {
            TaskState::Cancelling => notices.push(Notice::Cancelling(format!(
                "{} {}: {}",
                pending.animation.symbol(),
                i18n::tr!(Cancelling),
                pending.title
            ))),
            TaskState::Running if pending.feedback.visible() => {
                notices.push(Notice::Running(format!(
                    "{} {}: {}",
                    pending.animation.symbol(),
                    i18n::tr!(RunningAlt),
                    pending.title
                )))
            }
            TaskState::Running => {}
        }
    }
    if matches!(screen.mode, Mode::CommitMessage { .. })
        && let Some(label) = screen.commit_generator.label()
    {
        notices.push(Notice::Running(label));
    }
    if let Some(notice) = repository_activity(screen) {
        notices.push(notice);
    }
    if let Some(task) = &screen.check_task {
        notices.push(match task.state() {
            TaskState::Running => Notice::Running(task.summary()),
            TaskState::Cancelling => Notice::Cancelling(task.summary()),
        });
    } else if matches!(screen.snapshot.activity, Activity::Running) {
        notices.push(Notice::Running(i18n::tr!(CheckingAltAlt).into()));
    }
    notices
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
    let notices = activities(screen);
    let pending = screen
        .tasks
        .records
        .iter()
        .filter(|record| record.pending())
        .count();
    if notices.is_empty() {
        let text = if !screen.message.is_empty()
            && !screen
                .tasks
                .records
                .iter()
                .any(|record| record.active() || record.pending())
        {
            screen.message.lines().next().unwrap_or("").to_owned()
        } else {
            screen.tasks.footer_text()
        };
        return fit(
            &Line::styled(
                safe(&text),
                Style::default().fg(if pending > 0 { WARN } else { MUTED }),
            ),
            width,
        );
    }
    let mut main = Line::default();
    for notice in notices {
        if !main.spans.is_empty() {
            main.spans
                .push(Span::styled(" · ", Style::default().fg(MUTED)));
        }
        main.spans.push(notice.span());
    }
    let mut tail = Line::default();
    if pending > 0 {
        tail.spans.push(Span::styled(
            format!("{pending} {}", i18n::tr!(TasksPending)),
            Style::default().fg(WARN),
        ));
    }
    let mut full_tail = tail.clone();
    if pending > 0 {
        full_tail.spans.push(Span::raw(" · "));
    }
    full_tail.spans.push(Span::styled(
        i18n::tr!(TasksOpenShortcut),
        Style::default().fg(MUTED),
    ));
    // 窄屏先省略任务快捷键，再缩短进度；待处理提醒优先保留。
    if main.width() + 3 + full_tail.width() <= width {
        tail = full_tail;
    }
    if tail.spans.is_empty() {
        return fit(&main, width);
    }
    if tail.width() + 4 > width {
        return fit(
            &Line::styled(format!("! {pending}"), Style::default().fg(WARN)),
            width,
        );
    }
    let mut result = fit(&main, width - tail.width() - 3);
    result
        .spans
        .push(Span::styled(" · ", Style::default().fg(MUTED)));
    result.spans.extend(tail.spans);
    result
}
