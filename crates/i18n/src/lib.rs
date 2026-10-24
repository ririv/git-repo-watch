//! Typed bilingual messages and process-wide system language selection.
mod command;
pub use command::CommandSummary;
mod catalog;
mod system;
pub use catalog::Message;
#[doc(hidden)]
pub use identity as __identity;
use std::{cell::Cell, marker::PhantomData, rc::Rc, sync::OnceLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    English,
    Chinese,
}
impl Language {
    pub fn from_locale(locale: &str) -> Self {
        let primary = locale
            .trim()
            .split(['-', '_', '.', '@'])
            .next()
            .unwrap_or("");
        if primary.eq_ignore_ascii_case("zh") {
            Self::Chinese
        } else {
            Self::English
        }
    }
}

static LANGUAGE: OnceLock<Language> = OnceLock::new();
thread_local! { static OVERRIDE: Cell<Option<Language>> = const { Cell::new(None) }; }

pub fn language() -> Language {
    OVERRIDE
        .get()
        .unwrap_or_else(|| *LANGUAGE.get_or_init(system::detect))
}

/// Reject invalid explicit overrides before running commands or opening the terminal.
pub fn validate_environment() -> Result<(), &'static str> {
    match std::env::var("APP_LANG") {
        Ok(value) if value == "en" || value == "zh" => Ok(()),
        Ok(_) | Err(std::env::VarError::NotUnicode(_)) => Err("APP_LANG must be en or zh"),
        Err(std::env::VarError::NotPresent) => Ok(()),
    }
}

/// A thread-bound locale scope, useful for isolated renderers and deterministic tests.
pub struct LanguageScope {
    previous: Option<Language>,
    thread_bound: PhantomData<Rc<()>>,
}
pub fn using(language: Language) -> LanguageScope {
    LanguageScope {
        previous: OVERRIDE.replace(Some(language)),
        thread_bound: PhantomData,
    }
}
impl Drop for LanguageScope {
    fn drop(&mut self) {
        OVERRIDE.set(self.previous);
    }
}

#[macro_export]
macro_rules! tr {
    ($message:ident) => {
        $crate::Message::$message.text()
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chinese_locale_variants_and_english_default() {
        for locale in [
            "zh",
            "zh-CN",
            "zh_TW.UTF-8",
            "zh-Hant-HK",
            "ZH_cn",
            "zh@calendar=gregorian",
        ] {
            assert_eq!(Language::from_locale(locale), Language::Chinese);
        }
        for locale in [
            "en",
            "en_US.UTF-8",
            "fr-FR",
            "ja_JP",
            "C",
            "POSIX",
            "",
            "zhang",
        ] {
            assert_eq!(Language::from_locale(locale), Language::English);
        }
    }

    #[test]
    fn language_scopes_are_nested_and_thread_local() {
        let _english = using(Language::English);
        assert_eq!(tr!(Settings), "Settings");
        {
            let _chinese = using(Language::Chinese);
            assert_eq!(tr!(Settings), "设置");
            std::thread::spawn(|| {
                let _english = using(Language::English);
                assert_eq!(tr!(Settings), "Settings");
            })
            .join()
            .unwrap();
            assert_eq!(tr!(Settings), "设置");
        }
        assert_eq!(tr!(Settings), "Settings");
    }

    #[test]
    fn both_languages_preserve_format_arguments() {
        let _english = using(Language::English);
        assert_eq!(
            tf!(AheadOfRemoteByCommitsBehindBy, "feature", 3, 2),
            "feature: ahead of remote by 3 commits, behind by 2 commits"
        );
        let _chinese = using(Language::Chinese);
        assert_eq!(
            tf!(AheadOfRemoteByCommitsBehindBy, "feature", 3, 2),
            "feature：领先远程 3 个提交，落后远程 2 个提交"
        );
    }
}
