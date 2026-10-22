//! Command routing and backward-compatible machine output.
use anyhow::{Result, bail, ensure};
use engine::{
    cancel::{Cancel, Cancelled},
    discovery,
    domain::*,
    paths,
    scheduler::{Manager, Registration, Schedule},
    service::{Activity, CachedReport, ConfigEdit, Event, ScanMode, Scope, Service, Snapshot},
    state,
};
use std::ffi::OsString;
pub const HELP: i18n::Message = i18n::Message::CliHelp;
pub fn execute(args: Vec<OsString>) -> i32 {
    if let Err(error) = i18n::validate_environment() {
        eprintln!("{error}");
        return 2;
    }
    let cancel = Cancel::default();
    let signal = cancel.clone();
    if let Err(e) = ctrlc::set_handler(move || signal.cancel()) {
        eprintln!("{}", i18n::tf!(UnableToRegisterExitSignalE, e = e));
        return 2;
    }
    match run(args, &cancel) {
        Ok(code) => code,
        Err(e) if e.downcast_ref::<Cancelled>().is_some() || cancel.is_cancelled() => {
            eprintln!("{}", i18n::tr!(OperationCancelled));
            130
        }
        Err(e) => {
            eprintln!("{}", i18n::tf!(ErrorE, e = e));
            2
        }
    }
}
pub fn run(args: Vec<OsString>, cancel: &Cancel) -> Result<i32> {
    i18n::validate_environment().map_err(anyhow::Error::msg)?;
    let mut args = args
        .into_iter()
        .map(|s| {
            s.into_string()
                .map_err(|_| anyhow::anyhow!(i18n::tr!(ArgumentsMustBeUtf8)))
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
    let store = if args.first().is_some_and(|a| a == "--home") {
        ensure!(args.len() >= 2, i18n::tr!(HomeRequiresAPath));
        let home = paths::expand_home(&args[1], manager.user_home())?;
        args.drain(..2);
        ensure!(home.is_absolute(), i18n::tr!(HomeMustBeAnAbsolutePath));
        state::Store::custom(home)
    } else {
        manager.default_store()
    };
    let service = Service::new(store, manager);
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
            tui::run(service, cancel.clone())?;
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
                i18n::tr!(StatusOnlyAcceptsJson)
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
                println!("{}", i18n::tr!(NoCompleteReportYetWaitForThe));
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
        "init" => {
            none(&args)?;
            service.store.initialize()?;
            json(&service.store.config_path())?;
            Ok(0)
        }
        "paths" => {
            none(&args)?;
            json(
                &serde_json::json!({"config":service.store.config_path(),"database":service.store.database_path(),"logs":service.store.log_dir()}),
            )?;
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
            ensure!(
                !args.is_empty(),
                i18n::tr!(AddRequiresAtLeastOneRepositoryPath)
            );
            let paths = args
                .iter()
                .map(|value| paths::expand_home(value, service.scheduler.user_home()))
                .collect::<Result<Vec<_>>>()?;
            let n = service.add(&paths)?;
            println!("{}", i18n::tf!(AddedNRepositories, n = n));
            Ok(0)
        }
        "remove" | "exclude" | "unexclude" | "interval" | "autopush" | "concurrency"
        | "startup-check" => {
            let value = one(&args)?;
            let edit = match command.as_str() {
                "remove" => {
                    ConfigEdit::Remove(paths::expand_home(value, service.scheduler.user_home())?)
                }
                "exclude" => {
                    ConfigEdit::Exclude(paths::expand_home(value, service.scheduler.user_home())?)
                }
                "unexclude" => {
                    ConfigEdit::Unexclude(paths::expand_home(value, service.scheduler.user_home())?)
                }
                "interval" => ConfigEdit::Interval(state::interval(value)?),
                "autopush" => ConfigEdit::Policy(value.parse()?),
                "concurrency" => ConfigEdit::Concurrency(value.parse()?),
                "startup-check" => {
                    ensure!(
                        matches!(value, "true" | "false"),
                        i18n::tr!(BooleanArgumentRequired)
                    );
                    ConfigEdit::StartupCheck(value.parse()?)
                }
                _ => unreachable!(),
            };
            service.configure(edit)?;
            println!(
                "{}",
                if command == "startup-check" {
                    i18n::tr!(StartupCheckSavedNextLaunch)
                } else {
                    i18n::tr!(ConfigurationSavedApplySchedule)
                }
            );
            Ok(0)
        }
        "scan" | "import-tree" => {
            let root = paths::expand_home(one(&args)?, service.scheduler.user_home())?;
            let report = service.scan(
                &root,
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
                "{}",
                i18n::tf!(
                    FoundRepositoriesFailedCouldNotBeRead,
                    report.repositories.len(),
                    failed = failed
                )
            );
            Ok(i32::from(failed > 0))
        }
        "check" | "scheduled-check" => {
            if command == "scheduled-check" {
                none(&args)?;
            } else {
                ensure!(args.len() <= 1, i18n::tr!(CheckAtMostOnePath));
            }
            let emit = |event| match event {
                Event::Checking(p) => println!("{}", i18n::tf!(Checking, p.display())),
                Event::Checked { result: r, .. } => match serde_json::to_string_pretty(&r) {
                    Ok(s) => println!("{s}"),
                    Err(e) => eprintln!("{}", i18n::tf!(UnableToPrintResultE, e = e)),
                },
                Event::NotificationProblem(n) => {
                    eprintln!("{}", i18n::tf!(SystemNotificationFailedN, n = n))
                }
            };
            let report = if command == "scheduled-check" {
                service.run_scheduled(cancel, emit)?
            } else if let Some(path) = args.first() {
                service.check_repository(
                    &state::canonical(&paths::expand_home(path, service.scheduler.user_home())?)?,
                    cancel,
                    emit,
                )?
            } else {
                service.run(cancel, emit)?
            };
            println!(
                "{}",
                i18n::tf!(
                    CheckCompleteRepositories,
                    report.status,
                    report.repositories.len()
                )
            );
            Ok(i32::from(report.status != ReportStatus::Clean))
        }
        "schedule" => {
            let operation = match one(&args)? {
                "enable" => engine::scheduler::Control::Enable,
                "disable" => engine::scheduler::Control::Disable,
                "apply" => engine::scheduler::Control::Apply,
                _ => bail!(i18n::tr!(InvalidScheduleOperation)),
            };
            service.control_schedule(operation, cancel)?;
            println!(
                "{}",
                match operation {
                    engine::scheduler::Control::Enable => i18n::tr!(ScheduleEnabledNoCheck),
                    engine::scheduler::Control::Disable => i18n::tr!(ScheduleDisabledPreserved),
                    engine::scheduler::Control::Apply => i18n::tr!(ScheduleAppliedPreserved),
                }
            );
            Ok(0)
        }
        "migrate" => {
            if args == ["config"] {
                json(&service.store.migrate_configuration(cancel)?)?;
            } else if args.is_empty() {
                json(&service.store.migrate(cancel)?)?;
            } else {
                ensure!(
                    args.len() == 2 && args[0] == "--from",
                    i18n::tr!(MigrateFromUsage)
                );
                let source = paths::expand_home(&args[1], service.scheduler.user_home())?;
                ensure!(source.is_absolute(), i18n::tr!(HomeMustBeAnAbsolutePath));
                json(
                    &service
                        .store
                        .migrate_from(&state::Store::custom(source), cancel)?,
                )?;
            }
            Ok(0)
        }
        "install" => {
            none(&args)?;
            service.install(cancel)?;
            println!(
                "{}",
                i18n::tf!(
                    InstalledCommandAndSystemSchedule,
                    service.store.config()?.interval_seconds as f64 / 3600.0
                )
            );
            Ok(0)
        }
        "uninstall" => {
            none(&args)?;
            let _lock = service.store.lock()?;
            service.scheduler.uninstall(&service.store, cancel)?;
            println!(
                "{}",
                i18n::tr!(ScheduleRemovedConfigurationReportsAndCommandRetained)
            );
            Ok(0)
        }
        _ => bail!(i18n::tf!(UnknownCommandShowHelp, command = command)),
    }
}
fn none(args: &[String]) -> Result<()> {
    ensure!(
        args.is_empty(),
        i18n::tr!(ThisCommandDoesNotAcceptPositionalArguments)
    );
    Ok(())
}
fn one(args: &[String]) -> Result<&str> {
    ensure!(args.len() == 1, i18n::tr!(ThisCommandRequiresOneArgument));
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
                ensure!(
                    filter == Filter::All,
                    i18n::tr!(ReportFiltersAreMutuallyExclusive)
                );
                filter = match value.as_str() {
                    "--problems" => Filter::Problems,
                    "--no-upstream" => Filter::WithoutUpstream,
                    _ => Filter::Pushes,
                }
            }
            _ => bail!(i18n::tf!(UnknownReportArgumentValue, value = value)),
        }
    }
    Ok((machine, filter))
}
pub fn local_time(value: &str) -> String {
    match chrono::DateTime::parse_from_rfc3339(value) {
        Ok(time) => time
            .with_timezone(&chrono::Local)
            .format("%Y-%m-%d %H:%M:%S")
            .to_string(),
        Err(_) => i18n::tf!(InvalidReportTimestampValue, value = value),
    }
}
pub fn render_status(s: &Snapshot) -> String {
    let mut text = i18n::tf!(
        SystemScheduleCheckProcessConfiguredIntervalEvery,
        if s.registration == Registration::Loaded {
            i18n::tr!(Loaded)
        } else {
            i18n::tr!(NotLoaded)
        },
        if matches!(s.activity, Activity::Running) {
            i18n::tr!(Running)
        } else {
            i18n::tr!(Idle)
        },
        s.configured_interval_seconds as f64 / 3600.0,
        s.repository_count,
        s.max_concurrency,
        s.push_policy
    );
    text.push_str(&i18n::tf!(
        StartupCheckStatus,
        if s.check_on_startup {
            i18n::tr!(TaskEnabled)
        } else {
            i18n::tr!(TaskDisabled)
        }
    ));
    match s.schedule {
        Schedule::Missing => text.push_str(i18n::tr!(SystemScheduleFileMissing)),
        Schedule::Saved { interval_seconds } => {
            text.push_str(&i18n::tf!(
                ScheduledIntervalEveryHours,
                interval_seconds as f64 / 3600.0
            ));
            if interval_seconds != s.configured_interval_seconds {
                text.push_str(i18n::tr!(IntervalNotAppliedApplySchedule))
            }
        }
    }
    for excluded in &s.excluded_roots {
        text.push_str(&i18n::tf!(ExcludedDirectory, excluded.display()))
    }
    match &s.report {
        CachedReport::Missing => text.push_str(i18n::tr!(LatestReportNoCompleteReportYet)),
        CachedReport::Available { report, scope } => {
            let stats = Stats::of(report);
            text.push_str(&i18n::tf!(
                LastCheckLatestResultCleanNeedAttention,
                local_time(&report.finished_at),
                stats.clean,
                stats.attention,
                stats.failed,
                stats.pushed,
                stats.push_failed,
                stats.no_upstream,
                stats.no_upstream_repos
            ));
            if stats.failed > 0 {
                text.push_str(&i18n::tf!(
                    OtherRepositoriesFailedToCheckTheseCounts,
                    stats.failed
                ))
            }
            if stats.notice_failed > 0 {
                text.push_str(&i18n::tf!(
                    SystemNotificationsFailedSeePushReport,
                    stats.notice_failed
                ))
            }
            if report.push_policy == PushPolicy::Disabled
                && s.push_policy == PushPolicy::FastForward
            {
                text.push_str(i18n::tr!(LatestReportDidNotPushScheduledPolicy))
            }
            if report.status == ReportStatus::Unconfigured {
                text.push_str(i18n::tr!(NoRepositoriesWereConfiguredForThisCheck))
            }
            if matches!(scope, Scope::Outdated) {
                text.push_str(i18n::tr!(ReportScopeDiffersFromTheCurrentRepository))
            }
        }
    }
    text
}
pub fn render_report(report: &Report, filter: Filter) -> String {
    let mut text = i18n::tf!(CheckedAt, local_time(&report.finished_at));
    if let Some(time) = &report.classification_updated_at {
        text.push_str(&i18n::tf!(ClassificationUpdatedAt, local_time(time)))
    }
    for result in report.repositories.iter().filter(|r| filter.matches(r)) {
        text.push_str(&format!(
            "[{}] {}\n",
            if filter == Filter::WithoutUpstream {
                i18n::tr!(NoRemoteUpstream)
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
                text.push_str(&i18n::tf!(SystemNotificationFailedError, error = error))
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
                Publication::Pushed { target } => text.push_str(&i18n::tf!(
                    PushedCommits,
                    target.branch,
                    target.remote,
                    target.remote_ref,
                    target.ahead_commits
                )),
                Publication::Failed {
                    target,
                    phase,
                    error,
                } => text.push_str(&i18n::tf!(
                    PushFailedPhaseError,
                    target.branch,
                    error = error,
                    phase = phase
                )),
            }
        }
    }
    if matches!(filter, Filter::All | Filter::WithoutUpstream)
        && let Some(branches) = result.branches()
    {
        for branch in branches {
            text.push_str(&i18n::tf!(
                CountedSeparately,
                branch.branch,
                if branch.reason == UpstreamReason::LocalUpstream {
                    i18n::tr!(UpstreamIsALocalBranch)
                } else {
                    i18n::tr!(UpstreamNotConfigured)
                }
            ))
        }
    }
    text
}
