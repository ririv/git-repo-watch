//! Editor launches keep the current view intact and transfer terminal ownership only when needed.
use crate::{Effect, Outcome, Screen};
use anyhow::Result;
use engine::{
    editor::{self, Interface, Plan, Preference},
    service::ConfigEdit,
};
use std::{path::PathBuf, sync::Arc};
#[derive(Debug, Clone, PartialEq, Eq)]
struct Request {
    root: PathBuf,
    target: PathBuf,
    preference: Preference,
}
enum Prepared {
    Desktop(engine::desktop::Launch),
    Terminal(Plan),
    #[cfg(test)]
    Simulated,
}
#[cfg(test)]
type Sink = Arc<dyn Fn(&std::path::Path, &Preference) -> Result<()> + Send + Sync>;
#[derive(Default)]
pub(crate) struct Controller {
    worker: crate::reader::Worker<Request, Prepared>,
    terminal: Option<Plan>,
    launches: Vec<engine::desktop::Launch>,
    #[cfg(test)]
    sink: Option<Sink>,
}
impl Controller {
    fn submit(
        &mut self,
        root: PathBuf,
        target: PathBuf,
        preference: Preference,
        service: &Arc<crate::Service>,
    ) -> Result<()> {
        anyhow::ensure!(
            !self.worker.pending() && self.terminal.is_none() && self.launches.len() < 32,
            i18n::tr!(ExternalEditorOpening)
        );
        let request = Request {
            root,
            target,
            preference,
        };
        let requested = request.clone();
        let service = Arc::clone(service);
        #[cfg(test)]
        let sink = self.sink.clone();
        self.worker.request(request, move |cancel| {
            #[cfg(test)]
            if let Some(sink) = sink {
                sink(&requested.target, &requested.preference)?;
                return Ok(Prepared::Simulated);
            }
            let environment = editor::Environment::capture(service.scheduler.user_home());
            let plan = editor::prepare(
                &service.git,
                &requested.preference,
                &requested.target,
                &requested.root,
                &environment,
                &cancel,
            )?;
            match plan.interface {
                Interface::Desktop => plan.launch(&cancel).map(Prepared::Desktop),
                Interface::Terminal => Ok(Prepared::Terminal(plan)),
            }
        });
        Ok(())
    }
    fn poll(&mut self) -> Option<Result<Option<PathBuf>, String>> {
        let mut index = 0;
        while index < self.launches.len() {
            match self.launches[index].poll() {
                Ok(None) => index += 1,
                Ok(Some(())) => {
                    self.launches.swap_remove(index);
                }
                Err(error) => {
                    self.launches.swap_remove(index);
                    return Some(Err(format!("{error:#}")));
                }
            }
        }
        let (request, result) = self.worker.poll()?;
        Some(result.map(|prepared| match prepared {
            Prepared::Desktop(launch) => {
                self.launches.push(launch);
                Some(request.target)
            }
            Prepared::Terminal(plan) => {
                self.terminal = Some(plan);
                None
            }
            #[cfg(test)]
            Prepared::Simulated => Some(request.target),
        }))
    }
    pub fn take_terminal(&mut self) -> Option<Plan> {
        self.terminal.take()
    }
    pub fn shutdown(&mut self) {
        self.worker.clear();
        self.terminal = None;
    }
    #[cfg(test)]
    pub fn with_sink(
        sink: impl Fn(&std::path::Path, &Preference) -> Result<()> + Send + Sync + 'static,
    ) -> Self {
        Self {
            sink: Some(Arc::new(sink)),
            ..Self::default()
        }
    }
    #[cfg(test)]
    pub fn pending(&self) -> bool {
        self.worker.pending()
    }
}
impl Screen {
    pub(crate) fn select_editor(&mut self, editor: Preference) {
        self.start(
            i18n::tr!(ExternalEditorSetting),
            Effect::Configuration,
            move |service, cancel, _| {
                service.configure_with_cancel(ConfigEdit::Editor(editor), &cancel)?;
                Ok(Outcome::Text(i18n::tr!(PreferenceSaved).into()))
            },
        );
    }
    pub(crate) fn open_in_editor(
        &mut self,
        root: Option<PathBuf>,
        selection: &crate::clipboard::Selection,
    ) {
        let Some(root) = root else {
            self.message = i18n::tr!(SelectARepositoryFirst).into();
            return;
        };
        let Some(target) = selection.editor_target(&root) else {
            self.message = i18n::tr!(ExternalEditorSelectionRequired).into();
            return;
        };
        if let Err(error) =
            self.external_editor
                .submit(root, target, self.config.editor.clone(), &self.service)
        {
            self.message = format!("{error:#}");
        }
    }
    pub(crate) fn poll_external_editor(&mut self) -> bool {
        let Some(result) = self.external_editor.poll() else {
            return false;
        };
        match result {
            Ok(Some(path)) => self.message = i18n::tf!(ExternalEditorOpened, path.display()),
            Ok(None) => {}
            Err(error) => self.message = format!("{}: {error}", i18n::tr!(ExternalEditorFailed)),
        }
        true
    }
}
