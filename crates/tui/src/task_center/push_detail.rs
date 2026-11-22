//! Display successful pushes and read the former Debug output from task history.
use engine::domain::PushTarget;

pub(super) fn from_target(target: &PushTarget) -> String {
    summary(
        &target.branch,
        &target.remote,
        &target.remote_ref,
        &target.local_oid,
        target.ahead_commits,
    )
}

pub(crate) fn from_legacy(detail: &str) -> Option<String> {
    let target = detail
        .strip_prefix("Pushed {\n    target: PushTarget {\n")?
        .strip_suffix("    },\n}")?;
    let field = |name: &str| {
        target.lines().find_map(|line| {
            let (key, value) = line.strip_prefix("        ")?.split_once(": ")?;
            (key == name).then_some(value)?.strip_suffix(',')
        })
    };
    Some(summary(
        &debug_string(field("branch")?)?,
        &debug_string(field("remote")?)?,
        &debug_string(field("remote_ref")?)?,
        &debug_string(field("local_oid")?)?,
        field("ahead_commits")?.parse().ok()?,
    ))
}

pub(super) fn debug_string(quoted: &str) -> Option<String> {
    let mut input = quoted.strip_prefix('"')?.strip_suffix('"')?.chars();
    let mut result = String::new();
    while let Some(character) = input.next() {
        let character = if character == '\\' {
            match input.next()? {
                escaped @ ('"' | '\\') => escaped,
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                '0' => '\0',
                'u' if input.next()? == '{' => {
                    let (hex, remainder) = input.as_str().split_once('}')?;
                    input = remainder.chars();
                    char::from_u32(u32::from_str_radix(hex, 16).ok()?)?
                }
                _ => return None,
            }
        } else {
            character
        };
        result.push(character);
    }
    Some(result)
}

fn summary(branch: &str, remote: &str, reference: &str, oid: &str, ahead: usize) -> String {
    use crate::view::safe;

    let branch = safe(branch);
    let remote = safe(remote);
    let reference = safe(reference.trim_start_matches("refs/heads/"));
    let commit = safe(&oid.chars().take(12).collect::<String>());
    let mut lines = vec![format!(
        "{}: {branch} → {remote}/{reference}",
        i18n::tr!(PushTarget)
    )];
    if ahead > 0 {
        lines.push(i18n::tf!(PushedCommitsAlt, ahead));
    }
    lines.push(format!("{}: {commit}", i18n::tr!(PushNewTip)));
    lines.join("\n")
}
