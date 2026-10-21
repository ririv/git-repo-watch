//! Explicit command workflows over application services.
use super::super::*;
use anyhow::bail;
use gw_core::{
    local::files::IndexAction,
    service::{ConfigEdit, ScanMode},
    state,
};
use std::path::Path;
impl Screen {
    pub(crate) fn activate(
        &mut self,
        action: Action,
        path: Option<PathBuf>,
        root: &Cancel,
    ) -> Result<bool> {
        match action {
            Action::Quit => return Ok(true),
            Action::Settings => {
                self.page = Page::Settings;
                self.mode = Mode::Browsing;
                self.scroll = 0
            }
            Action::Help => {
                self.history.push(self.page);
                self.page = Page::Help;
                self.mode = Mode::Browsing;
                self.scroll = 0
            }
            Action::Refresh => {
                if let Some(session) = self.mode.file_session() {
                    let path = session.snapshot.path.clone();
                    let area = session.area;
                    self.read_files(path, area);
                } else if let Err(e) = self.reload(root) {
                    self.error("刷新失败", e)
                }
            }
            Action::Stage | Action::Unstage | Action::Diff => {
                if let Some(session) = self.mode.file_session() {
                    if action == Action::Diff {
                        self.show_diff();
                        return Ok(false);
                    }
                    if (action == Action::Stage && session.area == FileArea::Worktree)
                        || (action == Action::Unstage && session.area == FileArea::Index)
                    {
                        self.edit_files();
                        return Ok(false);
                    }
                }
                if let Some(path) = path {
                    self.read_files(
                        path,
                        if action == Action::Unstage {
                            FileArea::Index
                        } else {
                            FileArea::Worktree
                        },
                    )
                } else {
                    self.message = "请先选择仓库".into()
                }
            }
            Action::Commit | Action::Push => {
                if let Some(path) = path {
                    if action == Action::Commit {
                        self.start("读取暂存内容", Effect::ReadOnly, move |s, c, _| {
                            Ok(Outcome::CommitPrepared(s.prepare_commit(&path, &c)?))
                        })
                    } else {
                        self.start(
                            "核对当前分支与远程上游",
                            Effect::ReadOnly,
                            move |s, c, _| Ok(Outcome::PushPrepared(s.prepare_push(&path, &c)?)),
                        )
                    }
                } else {
                    self.message = "请先选择仓库".into()
                }
            }
            Action::Remove => {
                if let Some(path) = path {
                    self.mode = Mode::Confirm(Confirmation::Remove(path))
                } else {
                    self.message = "请先选择仓库".into()
                }
            }
            Action::CheckAll => self.mode = Mode::Confirm(Confirmation::CheckAll),
            Action::Apply => self.mode = Mode::Confirm(Confirmation::Apply),
            _ => {
                let value = match action {
                    Action::Interval => format!("{}s", self.config.interval_seconds),
                    Action::Autopush => self.config.push_policy.to_string(),
                    Action::Concurrency => self.config.max_concurrency.to_string(),
                    _ => String::new(),
                };
                self.mode = Mode::Input {
                    action,
                    editor: Editor::new(value),
                }
            }
        }
        Ok(false)
    }
    pub(crate) fn read_files(&mut self, path: PathBuf, area: FileArea) {
        self.start("读取仓库文件", Effect::ReadOnly, move |s, c, _| {
            Ok(Outcome::Files(s.files(&path, &c)?, area))
        })
    }
    pub(crate) fn show_diff(&mut self) {
        let Some(session) = self.mode.file_session().cloned() else {
            return;
        };
        let Some(change) = session.rows().get(session.cursor).map(|c| (*c).clone()) else {
            self.message = "当前区域没有文件".into();
            return;
        };
        self.start("读取文件差异", Effect::ReadOnly, move |s, c, _| {
            let content = s.diff(&session.snapshot, &change, &c)?;
            Ok(Outcome::Diff(
                session,
                format!("{}\n\n{content}", change.location.display()),
            ))
        })
    }
    pub(crate) fn edit_files(&mut self) {
        let Some(session) = self.mode.file_session().cloned() else {
            return;
        };
        let selected = session.selection();
        if selected.is_empty() {
            self.message = "当前区域没有可操作文件".into();
            return;
        }
        let action = if session.area == FileArea::Index {
            IndexAction::Unstage
        } else {
            IndexAction::Stage
        };
        self.start(
            if action == IndexAction::Stage {
                "暂存所选文件"
            } else {
                "取消暂存所选文件"
            },
            Effect::IndexWrite,
            move |s, c, _| {
                Ok(Outcome::Files(
                    s.edit_index(&session.snapshot, action, &selected, &c)?,
                    session.area,
                ))
            },
        )
    }
    pub(crate) fn confirm(&mut self, confirmation: Confirmation) {
        match confirmation {
            Confirmation::CheckAll => {
                self.start("检查全部仓库", Effect::Check, move |s, c, tx| {
                    let report = s.run(&c, |event| {
                        let _ = tx.send(WorkerEvent::Progress(event));
                    })?;
                    Ok(Outcome::Checked(report))
                })
            }
            Confirmation::Apply => self.start(
                "应用系统调度",
                Effect::Configuration,
                move |s, c, _| {
                    s.install(&c)?;
                    Ok(Outcome::Text("系统调度已应用；未触发检查".into()))
                },
            ),
            Confirmation::Remove(path) => {
                self.start("移除注册项", Effect::Configuration, move |s, _, _| {
                    s.configure(ConfigEdit::Remove(path))?;
                    Ok(Outcome::Text("已移除注册项，保留磁盘文件".into()))
                })
            }
            Confirmation::Configure { action, value } => self.start(
                command::description(action),
                Effect::Configuration,
                move |s, c, _| {
                    let message = match action {
                        Action::Add => format!("已添加 {} 个仓库", s.add(&[PathBuf::from(value)])?),
                        Action::Scan | Action::Import => {
                            let report = s.scan(
                                Path::new(&value),
                                if action == Action::Scan {
                                    ScanMode::Preview
                                } else {
                                    ScanMode::Import
                                },
                                &c,
                            )?;
                            format!(
                                "发现 {} 个仓库；详见 var/import-report.json",
                                report.repositories.len()
                            )
                        }
                        _ => {
                            let edit = match action {
                                Action::Exclude => ConfigEdit::Exclude(value.into()),
                                Action::Unexclude => ConfigEdit::Unexclude(value.into()),
                                Action::Interval => ConfigEdit::Interval(state::interval(&value)?),
                                Action::Autopush => ConfigEdit::Policy(value.parse()?),
                                Action::Concurrency => ConfigEdit::Concurrency(value.parse()?),
                                _ => bail!("此命令不接受配置输入"),
                            };
                            s.configure(edit)?;
                            "配置已保存；间隔修改后需应用调度".into()
                        }
                    };
                    Ok(Outcome::Text(message))
                },
            ),
        }
    }
}
