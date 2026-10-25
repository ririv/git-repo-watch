//! Persisted pointer ownership with independent, serialized preference writes.
use crate::Screen;
use anyhow::Result;
use crossterm::{
    event::{
        DisableMouseCapture, EnableMouseCapture, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
    },
    execute,
};
use engine::{cancel::Cancel, service::ConfigEdit, state::InteractionMode};
use std::{
    io::{self, Write},
    sync::Arc,
    thread::{self, JoinHandle},
};

pub(crate) fn label(mode: InteractionMode) -> &'static str {
    match mode {
        InteractionMode::Application => i18n::tr!(InteractionApplication),
        InteractionMode::TextSelection => i18n::tr!(InteractionText),
    }
}
pub(crate) fn apply(mode: InteractionMode, output: &mut impl Write) -> io::Result<()> {
    match mode {
        InteractionMode::Application => execute!(output, EnableMouseCapture),
        InteractionMode::TextSelection => execute!(output, DisableMouseCapture),
    }
}

#[derive(Default)]
pub(crate) enum Writer {
    #[default]
    Idle,
    Saving {
        target: InteractionMode,
        latest: InteractionMode,
        cancel: Cancel,
        handle: JoinHandle<Result<()>>,
    },
}
impl Writer {
    fn requested(&self, current: InteractionMode) -> InteractionMode {
        match self {
            Self::Idle => current,
            Self::Saving { latest, .. } => *latest,
        }
    }
    #[cfg(test)]
    pub(crate) fn pending(&self) -> bool {
        matches!(self, Self::Saving { .. })
    }
    fn submit(
        &mut self,
        service: Arc<engine::service::Service>,
        mode: InteractionMode,
        current: InteractionMode,
    ) {
        match self {
            Self::Saving { latest, .. } => *latest = mode,
            Self::Idle if mode == current => {}
            Self::Idle => {
                let cancel = Cancel::default();
                let task_cancel = cancel.clone();
                let language = i18n::language();
                let handle = thread::spawn(move || {
                    let _language = i18n::using(language);
                    service.configure_with_cancel(ConfigEdit::Interaction(mode), &task_cancel)
                });
                *self = Self::Saving {
                    target: mode,
                    latest: mode,
                    cancel,
                    handle,
                };
            }
        }
    }
    fn poll(&mut self) -> Option<(InteractionMode, InteractionMode, Result<()>)> {
        if !matches!(self, Self::Saving { handle, .. } if handle.is_finished()) {
            return None;
        }
        let Self::Saving {
            target,
            latest,
            handle,
            ..
        } = std::mem::take(self)
        else {
            unreachable!()
        };
        let result = handle
            .join()
            .unwrap_or_else(|_| Err(anyhow::anyhow!(i18n::tr!(BackgroundWorkerPanicked))));
        Some((target, latest, result))
    }
    pub(crate) fn shutdown(&mut self) {
        if let Self::Saving { cancel, handle, .. } = std::mem::take(self) {
            cancel.cancel();
            let _ = handle.join();
        }
    }
}
impl Screen {
    pub(crate) fn interaction_key(&mut self, key: KeyEvent) -> bool {
        if key.code != KeyCode::Char('g') || key.modifiers != KeyModifiers::CONTROL {
            return false;
        }
        if key.kind == KeyEventKind::Press {
            let current = self
                .interaction_save
                .requested(self.config.interaction_mode);
            self.set_interaction(current.next());
        }
        true
    }
    pub(crate) fn set_interaction(&mut self, mode: InteractionMode) {
        self.drag = None;
        self.clicks.reset();
        self.interaction_save
            .submit(self.service.clone(), mode, self.config.interaction_mode);
    }
    pub(crate) fn poll_interaction(&mut self) -> bool {
        let Some((target, latest, result)) = self.interaction_save.poll() else {
            return false;
        };
        match result {
            Ok(()) => self.config.interaction_mode = target,
            Err(error) => self.message = format!("{}: {error:#}", i18n::tr!(InteractionSaveFailed)),
        }
        // 快速连续切换只保存最终意图；前一次写入结束后串行处理，避免乱序覆盖。
        if latest != target {
            self.set_interaction(latest);
        }
        true
    }
    pub(crate) fn interaction_menu(&mut self) {
        self.open_option_menu(
            crate::command::options::Kind::Interaction,
            self.selected_path(),
            crate::clipboard::Selection::None,
        );
    }
}
