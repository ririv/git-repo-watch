//! Render an existing repository view into a buffer without taking over a terminal.
use crate::{
    Mode, Service, Start, changes, files, history, overview, remote_info, startup, view, workspace,
};
use anyhow::{Context, Result, bail, ensure};
use engine::cancel::Cancel;
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer, layout::Size};
use std::{
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Copy)]
pub enum View {
    Overview,
    Changes { file: usize },
    History { commit: usize },
}

pub struct Snapshot {
    pub buffer: Buffer,
    pub position: usize,
    pub total: usize,
}

/// Read and draw a repository view for a host that owns input and rendering.
pub fn render(service: Service, cancel: &Cancel, view: View, size: Size) -> Result<Snapshot> {
    ensure!(size.width > 0 && size.height > 0, "面板尺寸不能为空");
    let entry = Start::CurrentDirectory.resolve(cancel)?;
    let startup::Entry::Repository(_) = entry else {
        bail!("当前目录不在 Git 仓库中");
    };
    let mut screen = entry.open(service, cancel)?;
    screen.live.shutdown();
    let result = render_screen(&mut screen, cancel, view, size);
    screen.shutdown();
    result
}

fn render_screen(
    screen: &mut crate::Screen,
    cancel: &Cancel,
    view: View,
    size: Size,
) -> Result<Snapshot> {
    let deadline = Instant::now() + Duration::from_secs(5);
    screen.poll_remote_info();
    while screen.workspace.loading()
        || matches!(
            screen.overview.content,
            overview::Content::Loading(_) | overview::Content::Refreshing(_)
        )
        || matches!(screen.remote_info.content, remote_info::Content::Loading(_))
    {
        wait(cancel, deadline)?;
        screen.poll_workspace();
        screen.poll_remote_info();
        screen.overview.poll(&screen.service);
    }

    let (position, total) = match view {
        View::Overview => (0, 0),
        View::Changes { file } => {
            screen.switch_repository_section(workspace::Section::Worktree);
            let Mode::Files(files) = &mut screen.mode else {
                bail!("无法打开工作区文件列表");
            };
            let total = files.rows().len();
            let position = file.min(total.saturating_sub(1));
            if total > 0 {
                files.cursor = files::Cursor::File(position);
                files.set_layout(screen.file_layout);
                screen.show_diff();
                while matches!(&screen.mode, Mode::Changes(session)
                    if matches!(session.preview, changes::Preview::Loading(_)))
                {
                    wait(cancel, deadline)?;
                    screen.poll_changes();
                }
            }
            (position, total)
        }
        View::History { commit } => {
            screen.switch_repository_section(workspace::Section::History);
            screen
                .workspace
                .session_mut()
                .context("缺少仓库工作区")?
                .arrangement = workspace::Arrangement::Maximized;
            let Mode::History(view) = &mut screen.mode else {
                bail!("无法打开提交历史");
            };
            let history::View::Graph(graph) = view.as_mut() else {
                bail!("无法打开提交历史图");
            };
            let total = graph.records.len();
            let position = commit.min(total.saturating_sub(1));
            graph.state.select((total > 0).then_some(position));
            (position, total)
        }
    };

    let mut terminal =
        Terminal::new(TestBackend::new(size.width, size.height)).context("创建面板画布")?;
    terminal.draw(|frame| view::render_body(frame, screen, frame.area()))?;
    Ok(Snapshot {
        buffer: terminal.backend().buffer().clone(),
        position,
        total,
    })
}

fn wait(cancel: &Cancel, deadline: Instant) -> Result<()> {
    cancel.check()?;
    ensure!(Instant::now() < deadline, "读取仓库面板超时");
    thread::sleep(Duration::from_millis(5));
    Ok(())
}
