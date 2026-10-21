//! Configured refspec mappings restricted to remote-tracking destinations.
use anyhow::{Context, Result, ensure};
use git2::{ErrorCode, Repository};

#[derive(Debug)]
enum Refspec {
    Exclude(String),
    Map {
        source: String,
        destination: String,
        text: String,
    },
    SourceOnly,
}
#[derive(Debug)]
pub struct Remote {
    pub name: String,
    refspecs: Vec<Refspec>,
}
impl Remote {
    pub fn read(repo: &Repository, name: &str) -> Result<Self> {
        repo.find_remote(name)?;
        let config = repo.config()?;
        let mut refspecs = Vec::new();
        match config.multivar(&format!("remote.{name}.fetch"), None) {
            Ok(mut entries) => {
                while let Some(entry) = entries.next() {
                    let entry = entry?;
                    let text = entry.value().context(i18n::tr!(FetchRefspecNotUtf8))?;
                    let spec = if let Some(source) = text.strip_prefix('^') {
                        validate_pattern(source)?;
                        Refspec::Exclude(source.into())
                    } else {
                        let value = text.strip_prefix('+').unwrap_or(text);
                        match value.split_once(':') {
                            Some((source, destination)) => {
                                validate_pattern(source)?;
                                validate_pattern(destination)?;
                                ensure!(
                                    source.matches('*').count() == destination.matches('*').count(),
                                    i18n::tf!(InvalidFetchRefspec, text)
                                );
                                Refspec::Map {
                                    source: source.into(),
                                    destination: destination.into(),
                                    text: text.into(),
                                }
                            }
                            None => Refspec::SourceOnly,
                        }
                    };
                    refspecs.push(spec);
                }
            }
            Err(error) if error.code() == ErrorCode::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        Ok(Self {
            name: name.into(),
            refspecs,
        })
    }
    pub fn upstream(&self, source: &str) -> Result<String> {
        ensure!(!self.refspecs.iter().any(|spec| matches!(spec, Refspec::Exclude(pattern) if capture(pattern, source).is_some())), i18n::tf!(UpstreamExcludedFromFetch, self.name, source));
        let mut destinations = Vec::new();
        for spec in &self.refspecs {
            if let Refspec::Map {
                source: pattern,
                destination,
                ..
            } = spec
                && let Some(capture) = capture(pattern, source)
            {
                ensure!(
                    destination.starts_with("refs/remotes/"),
                    i18n::tf!(UnsupportedFetchDestination, destination)
                );
                let reference = destination.replace('*', capture);
                if !destinations.contains(&reference) {
                    destinations.push(reference);
                }
            }
        }
        ensure!(
            destinations.len() == 1,
            i18n::tf!(UpstreamFetchMappingNotUnique, self.name, source)
        );
        Ok(destinations.remove(0))
    }
    pub fn arguments(&self) -> Vec<&str> {
        self.refspecs
            .iter()
            .filter_map(|spec| match spec {
                Refspec::Map {
                    destination, text, ..
                } if destination.starts_with("refs/remotes/") => Some(text.as_str()),
                _ => None,
            })
            .collect()
    }
    pub fn exclusions(&self) -> impl Iterator<Item = &str> {
        self.refspecs.iter().filter_map(|spec| match spec {
            Refspec::Exclude(source) => Some(source.as_str()),
            _ => None,
        })
    }
    pub fn skipped(&self, repo: &Repository) -> Result<bool> {
        match repo
            .config()?
            .get_bool(&format!("remote.{}.skipFetchAll", self.name))
        {
            Ok(value) => Ok(value),
            Err(error) if error.code() == ErrorCode::NotFound => Ok(false),
            Err(error) => Err(error.into()),
        }
    }
}
fn validate_pattern(pattern: &str) -> Result<()> {
    ensure!(
        pattern.matches('*').count() <= 1
            && git2::Reference::is_valid_name(&pattern.replace('*', "wildcard")),
        i18n::tf!(InvalidFetchRefspec, pattern)
    );
    Ok(())
}
fn capture<'a>(pattern: &str, value: &'a str) -> Option<&'a str> {
    match pattern.split_once('*') {
        Some((prefix, suffix)) => value.strip_prefix(prefix)?.strip_suffix(suffix),
        None => (pattern == value).then_some(""),
    }
}

/// Reject overlapping pruning scopes before any remote-tracking references change.
pub fn validate_destinations(remotes: &[Remote]) -> Result<()> {
    let mut seen: Vec<(&str, &str, &str)> = Vec::new();
    for remote in remotes {
        for spec in &remote.refspecs {
            if let Refspec::Map {
                destination, text, ..
            } = spec
            {
                if !destination.starts_with("refs/remotes/") {
                    continue;
                }
                for (name, previous, previous_text) in &seen {
                    if *name == remote.name && *previous_text == text {
                        continue;
                    }
                    ensure!(
                        !overlap(previous, destination),
                        i18n::tf!(
                            OverlappingFetchDestinations,
                            name,
                            previous,
                            remote.name,
                            destination
                        )
                    );
                }
                seen.push((&remote.name, destination, text));
            }
        }
    }
    Ok(())
}
fn overlap(a: &str, b: &str) -> bool {
    match (a.split_once('*'), b.split_once('*')) {
        (Some((ap, az)), Some((bp, bz))) => {
            (ap.starts_with(bp) || bp.starts_with(ap)) && (az.ends_with(bz) || bz.ends_with(az))
        }
        (Some(_), None) => capture(a, b).is_some(),
        _ => capture(b, a).is_some(),
    }
}
