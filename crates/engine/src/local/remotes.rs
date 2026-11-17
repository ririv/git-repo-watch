//! Read configured remote endpoints without contacting servers or changing Git state.
use crate::{cancel::Cancel, local::open};
use anyhow::{Context, Result};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Fetch,
    Push,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    pub name: String,
    pub direction: Direction,
    pub url: String,
}

pub fn read(path: &Path, cancel: &Cancel) -> Result<Vec<Endpoint>> {
    cancel.check()?;
    let repo = open(path)?;
    let config = repo.config()?.snapshot()?;
    let names = repo.remotes()?;
    let mut result = Vec::new();
    for name in names.iter() {
        cancel.check()?;
        let name = name?.context(i18n::tr!(RemoteUrlIsNotUtf8))?;
        let mut fetch = Vec::new();
        for (key, direction) in [("url", Direction::Fetch), ("pushurl", Direction::Push)] {
            let mut entries = match config.multivar(&format!("remote.{name}.{key}"), None) {
                Ok(entries) => entries,
                Err(error) if error.code() == git2::ErrorCode::NotFound => continue,
                Err(error) => return Err(error.into()),
            };
            while let Some(entry) = entries.next() {
                cancel.check()?;
                let entry = entry?;
                let value = entry.value().context(i18n::tr!(RemoteUrlIsNotUtf8))?;
                let url = public_url(value);
                if direction == Direction::Fetch {
                    fetch.push(url.clone());
                }
                if direction == Direction::Push && fetch.contains(&url) {
                    continue;
                }
                result.push(Endpoint {
                    name: name.into(),
                    direction,
                    url,
                });
            }
        }
    }
    result.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(result)
}

// 地址只展示公开部分；HTTP 认证信息和查询参数不能进入界面或链接目标。
fn public_url(value: &str) -> String {
    let Some((scheme, rest)) = value.split_once("://") else {
        return value.to_owned();
    };
    let boundary = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = &rest[..boundary];
    let host = authority.rsplit_once('@').map_or_else(
        || authority.to_owned(),
        |(user, host)| {
            if matches!(scheme, "ssh" | "git+ssh") {
                format!("{}@{host}", user.split(':').next().expect("SSH user"))
            } else {
                host.to_owned()
            }
        },
    );
    let path = rest[boundary..]
        .split(['?', '#'])
        .next()
        .expect("URL path segment");
    format!("{scheme}://{host}{path}")
}
