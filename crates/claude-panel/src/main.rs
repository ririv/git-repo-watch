//! One read-only frame per invocation, so Claude Code owns the pane lifecycle.
mod frame;

use anyhow::{Context, Result, bail, ensure};
use engine::{cancel::Cancel, scheduler::Manager, service::Service, state::Store};
use ratatui::layout::Size;
use std::{env, path::PathBuf};
use tui::panel::View;

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    i18n::validate_environment().map_err(anyhow::Error::msg)?;
    let mut args = env::args().skip(1);
    let Some(first) = args.next() else {
        bail!("参数： [--home PATH] overview|changes|history WIDTH HEIGHT POSITION");
    };
    let (home, view) = if first == "--home" {
        let home = args.next().context("缺少配置目录")?;
        (
            Some(PathBuf::from(home)),
            args.next().context("缺少面板视图")?,
        )
    } else {
        (None, first)
    };
    let width: u16 = args.next().context("缺少面板宽度")?.parse()?;
    let height: u16 = args.next().context("缺少面板高度")?.parse()?;
    let position: usize = args.next().context("缺少选中项位置")?.parse()?;
    ensure!(args.next().is_none(), "面板参数过多");
    ensure!(
        (20..=160).contains(&width) && (10..=48).contains(&height),
        "面板尺寸超出范围"
    );
    let view = match view.as_str() {
        "overview" => View::Overview,
        "changes" => View::Changes { file: position },
        "history" => View::History { commit: position },
        _ => bail!("未知面板视图：{view}"),
    };
    let manager = Manager::new()?;
    let store = home.map_or_else(|| manager.default_store(), Store::custom);
    let service = Service::new(store, manager);
    let snapshot = tui::panel::render(service, &Cancel::default(), view, Size::new(width, height))?;
    serde_json::to_writer(std::io::stdout().lock(), &frame::Frame::from(snapshot))?;
    Ok(())
}
