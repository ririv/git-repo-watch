//! Command routing and backward-compatible machine output.
use anyhow::{Context, Result, bail, ensure};
use gw_core::{
    cancel::{Cancel, Cancelled},
    discovery,
    domain::*,
    scheduler::{Manager, Registration, Schedule},
    service::{Activity, CachedReport, ConfigEdit, Event, ScanMode, Scope, Service, Snapshot},
    state,
};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};
pub const HELP: &str = r#"gw：Git 仓库状态查询与配置管理

用法：gw [--home 绝对路径] [命令]
不传命令时进入 TUI；状态查询使用 gw status。

  tui                              打开交互界面，不自动检查
  status [--json]                   查看任务、配置和上次结果
  report [--json] [筛选]             读取完整报告
    --problems                      只看需关注或检查失败
    --no-upstream                   单独查看无远程上游的分支
    --pushes                        只看自动推送结果
  config                            查看配置 JSON
  list                              查看仓库列表
  exclusions                        查看排除目录
  add 路径...                       添加仓库根目录
  remove 路径                       移除仓库注册项
  exclude 路径                      排除目录并移除其下注册项
  unexclude 路径                    取消排除，不自动添加仓库
  scan 路径                         按 Git 忽略规则扫描，写报告
  import-tree 路径                  扫描并导入列表
  interval 6h                       保存频率（支持 s、m、h、d）
  autopush disabled|fast_forward     保存自动推送策略
  concurrency 4                     设置检查并发上限（1 至 32）
  check                             并行检查与配置的自动推送
  install                           安装命令并应用调度，不立即检查
  uninstall                         卸载定时任务，保留数据与命令
"#;
pub fn execute(args: Vec<OsString>) -> i32 {
    let cancel = Cancel::default();
    let signal = cancel.clone();
    if let Err(e) = ctrlc::set_handler(move || signal.cancel()) {
        eprintln!("无法注册退出信号：{e}");
        return 2;
    }
    match run(args, &cancel) {
        Ok(code) => code,
        Err(e) if e.downcast_ref::<Cancelled>().is_some() || cancel.is_cancelled() => {
            eprintln!("操作已取消");
            130
        }
        Err(e) => {
            eprintln!("错误：{e:#}");
            2
        }
    }
}
pub fn run(args: Vec<OsString>, cancel: &Cancel) -> Result<i32> {
    let mut args = args
        .into_iter()
        .map(|s| {
            s.into_string()
                .map_err(|_| anyhow::anyhow!("参数必须为 UTF-8"))
        })
        .collect::<Result<Vec<_>>>()?;
    if args
        .first()
        .is_some_and(|a| a == "--help" || a == "-h" || a == "help")
    {
        print!("{HELP}");
        return Ok(0);
    }
    let manager = Manager::new()?;
    let home = if args.first().is_some_and(|a| a == "--home") {
        ensure!(args.len() >= 2, "--home 缺少路径");
        let home = PathBuf::from(&args[1]);
        args.drain(..2);
        home
    } else {
        manager
            .read_location()
            .context("无法读取安装位置，请运行 ./install.sh 或显式传入 --home")?
            .home
    };
    ensure!(home.is_absolute(), "--home 必须是绝对路径");
    let service = Service::new(home, manager);
    let command = if args.is_empty() {
        "tui".to_owned()
    } else {
        args.remove(0)
    };
    match command.as_str() {
        "help" | "--help" | "-h" => {
            print!("{HELP}");
            Ok(0)
        }
        "tui" => {
            none(&args)?;
            gw_tui::run(service, cancel.clone())?;
            Ok(0)
        }
        "status" if args == ["--help"] || args == ["-h"] => {
            print!("{HELP}");
            Ok(0)
        }
        "report" if args == ["--help"] || args == ["-h"] => {
            print!("{HELP}");
            Ok(0)
        }
        "status" => {
            ensure!(
                args.is_empty() || args == ["--json"],
                "status 只接受 --json"
            );
            let s = service.snapshot(cancel)?;
            if args == ["--json"] {
                json(&s)?
            } else {
                print!("{}", render_status(&s))
            }
            Ok(0)
        }
        "report" => {
            let (machine, filter) = report_options(&args)?;
            let Some(mut report) = service.store.report()? else {
                println!("尚未生成完整报告；等待定时执行或显式运行 check");
                return Ok(1);
            };
            if machine {
                report.repositories.retain(|r| filter.matches(r));
                json(&report)?
            } else {
                print!("{}", render_report(&report, filter))
            }
            Ok(0)
        }
        "config" | "list" | "exclusions" => {
            none(&args)?;
            let c = service.store.config()?;
            match command.as_str() {
                "config" => json(&c)?,
                "list" => {
                    for p in c.repositories {
                        println!("{}", p.display())
                    }
                }
                _ => {
                    for p in c.excluded_roots {
                        println!("{}", p.display())
                    }
                }
            }
            Ok(0)
        }
        "add" => {
            ensure!(!args.is_empty(), "add 至少需要一个仓库路径");
            let paths = args.iter().map(PathBuf::from).collect::<Vec<_>>();
            let n = service.add(&paths)?;
            println!("已添加 {n} 个仓库");
            Ok(0)
        }
        "remove" | "exclude" | "unexclude" | "interval" | "autopush" | "concurrency" => {
            let value = one(&args)?;
            let edit = match command.as_str() {
                "remove" => ConfigEdit::Remove(value.into()),
                "exclude" => ConfigEdit::Exclude(value.into()),
                "unexclude" => ConfigEdit::Unexclude(value.into()),
                "interval" => ConfigEdit::Interval(state::interval(value)?),
                "autopush" => ConfigEdit::Policy(value.parse()?),
                "concurrency" => ConfigEdit::Concurrency(value.parse()?),
                _ => unreachable!(),
            };
            service.configure(edit)?;
            println!("配置已保存；间隔修改需运行 gw install 应用调度，不立即检查");
            Ok(0)
        }
        "scan" | "import-tree" => {
            let root = one(&args)?;
            let report = service.scan(
                Path::new(root),
                if command == "scan" {
                    ScanMode::Preview
                } else {
                    ScanMode::Import
                },
                cancel,
            )?;
            let failed = report
                .repositories
                .iter()
                .filter(|r| matches!(r, discovery::Record::Failed { .. }))
                .count();
            println!(
                "发现 {} 个仓库，其中 {failed} 个无法读取；详见 var/import-report.json",
                report.repositories.len()
            );
            Ok(i32::from(failed > 0))
        }
        "check" => {
            none(&args)?;
            let report = service.run(cancel, |event| match event {
                Event::Checking(p) => println!("正在检查：{}", p.display()),
                Event::Checked(r) => match serde_json::to_string_pretty(&r) {
                    Ok(s) => println!("{s}"),
                    Err(e) => eprintln!("无法输出结果：{e}"),
                },
                Event::NotificationProblem(n) => eprintln!("系统通知发送失败：{n:?}"),
            })?;
            println!(
                "检查完成：{:?}，共 {} 个仓库",
                report.status,
                report.repositories.len()
            );
            Ok(i32::from(report.status != ReportStatus::Clean))
        }
        "install" => {
            none(&args)?;
            service.install(cancel)?;
            println!(
                "已安装原生 gw 命令与系统任务，每 {} 小时检查一次；未触发检查",
                service.store.config()?.interval_seconds as f64 / 3600.0
            );
            Ok(0)
        }
        "uninstall" => {
            none(&args)?;
            let _lock = service.store.lock()?;
            service.scheduler.uninstall(cancel)?;
            println!("已卸载定时任务，保留配置、报告和管理命令");
            Ok(0)
        }
        _ => bail!("未知命令 {command:?}；运行 gw --help 查看帮助"),
    }
}
fn none(args: &[String]) -> Result<()> {
    ensure!(args.is_empty(), "该命令不接受位置参数");
    Ok(())
}
fn one(args: &[String]) -> Result<&str> {
    ensure!(args.len() == 1, "该命令要求一个参数");
    Ok(&args[0])
}
fn json(value: &impl serde::Serialize) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
pub fn report_options(args: &[String]) -> Result<(bool, Filter)> {
    let mut machine = false;
    let mut filter = Filter::All;
    for value in args {
        match value.as_str() {
            "--json" => machine = true,
            "--help" | "-h" => {
                print!("{HELP}");
            }
            "--problems" | "--no-upstream" | "--pushes" => {
                ensure!(filter == Filter::All, "报告筛选参数互斥");
                filter = match value.as_str() {
                    "--problems" => Filter::Problems,
                    "--no-upstream" => Filter::WithoutUpstream,
                    _ => Filter::Pushes,
                }
            }
            _ => bail!("未知报告参数：{value}"),
        }
    }
    Ok((machine, filter))
}
pub fn local_time(value: &str) -> String {
    match chrono::DateTime::parse_from_rfc3339(value) {
        Ok(time) => time
            .with_timezone(&chrono::Local)
            .format("%Y-%m-%d %H:%M:%S %Z")
            .to_string(),
        Err(_) => format!("报告时间格式无效：{value}"),
    }
}
pub fn render_status(s: &Snapshot) -> String {
    let mut text = format!(
        "系统任务：{}\n检查进程：{}\n配置频率：每 {} 小时\n仓库数量：{}\n检查并发：{}\n自动推送：{}\n",
        if s.registration == Registration::Loaded {
            "已加载"
        } else {
            "未加载"
        },
        if matches!(s.activity, Activity::Running) {
            "运行中"
        } else {
            "空闲"
        },
        s.configured_interval_seconds as f64 / 3600.0,
        s.repository_count,
        s.max_concurrency,
        s.push_policy
    );
    match s.schedule {
        Schedule::Missing => text.push_str("系统任务配置文件：不存在\n"),
        Schedule::Saved { interval_seconds } => {
            text.push_str(&format!(
                "系统任务频率：每 {} 小时\n",
                interval_seconds as f64 / 3600.0
            ));
            if interval_seconds != s.configured_interval_seconds {
                text.push_str("频率尚未应用，请运行 gw install\n")
            }
        }
    }
    for excluded in &s.excluded_roots {
        text.push_str(&format!("排除目录：{}\n", excluded.display()))
    }
    match &s.report {
        CachedReport::Missing => text.push_str("最近报告：尚未生成完整报告\n"),
        CachedReport::Available { report, scope } => {
            let stats = Stats::of(report);
            text.push_str(&format!("最近检查：{}\n最近结果：{} 个正常，{} 个需关注，{} 个检查失败\n最近自动推送：{} 个分支成功，{} 个分支失败\n未配置远程上游（单独统计）：{} 个分支，涉及 {} 个仓库\n",local_time(&report.finished_at),stats.clean,stats.attention,stats.failed,stats.pushed,stats.push_failed,stats.no_upstream,stats.no_upstream_repos));
            if stats.failed > 0 {
                text.push_str(&format!(
                    "另有 {} 个仓库检查失败，此项统计不完整\n",
                    stats.failed
                ))
            }
            if stats.notice_failed > 0 {
                text.push_str(&format!(
                    "系统通知发送失败：{} 条，详情见 gw report --pushes\n",
                    stats.notice_failed
                ))
            }
            if report.push_policy == PushPolicy::Disabled
                && s.push_policy == PushPolicy::FastForward
            {
                text.push_str("最近报告来自自动推送启用前；将在下次检查时应用\n")
            }
            if report.status == ReportStatus::Unconfigured {
                text.push_str("该轮检查未配置仓库\n")
            }
            if matches!(scope, Scope::Outdated) {
                text.push_str("报告范围与当前仓库列表不同，等待下一轮检查或显式运行 check\n")
            }
        }
    }
    text
}
pub fn render_report(report: &Report, filter: Filter) -> String {
    let mut text = format!("检查数据时间：{}\n", local_time(&report.finished_at));
    if let Some(time) = &report.classification_updated_at {
        text.push_str(&format!("分类规则更新时间：{}\n", local_time(time)))
    }
    for result in report.repositories.iter().filter(|r| filter.matches(r)) {
        text.push_str(&format!(
            "[{}] {}\n",
            if filter == Filter::WithoutUpstream {
                "未配置远程上游"
            } else {
                result.label()
            },
            result.path().display()
        ));
        text.push_str(&repository_detail(result, filter));
        for notice in &report.notifications {
            if let Notice::Failed { path, error, .. } = notice
                && path == result.path()
            {
                text.push_str(&format!("  - 系统通知发送失败：{error}\n"))
            }
        }
    }
    text
}
pub fn repository_detail(result: &RepositoryResult, filter: Filter) -> String {
    let mut text = String::new();
    match result {
        RepositoryResult::Attention { issues, .. }
            if filter != Filter::Pushes && filter != Filter::WithoutUpstream =>
        {
            for issue in issues {
                text.push_str(&format!("  - {}\n", issue.detail))
            }
        }
        RepositoryResult::Failed { error, .. } => text.push_str(&format!("  - {error}\n")),
        _ => {}
    }
    if filter != Filter::WithoutUpstream {
        for push in result.pushes() {
            match push {
                Publication::Pushed { target } => text.push_str(&format!(
                    "  - 已推送 {} → {}/{}，{} 个提交\n",
                    target.branch, target.remote, target.remote_ref, target.ahead_commits
                )),
                Publication::Failed {
                    target,
                    phase,
                    error,
                } => text.push_str(&format!(
                    "  - 推送失败 {}（{phase:?}）：{error}\n",
                    target.branch
                )),
            }
        }
    }
    if matches!(filter, Filter::All | Filter::WithoutUpstream)
        && let Some(branches) = result.branches()
    {
        for branch in branches {
            text.push_str(&format!(
                "  - {}：{}（单独统计）\n",
                branch.branch,
                if branch.reason == UpstreamReason::LocalUpstream {
                    "上游是本地分支"
                } else {
                    "未配置上游"
                }
            ))
        }
    }
    text
}
