//! Named parameters are separate candidates owned by a command.
use super::{Command, Input, options::Kind};
use crate::Editor;
use std::ops::Range;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Parameter {
    pub name: &'static str,
    pub kind: Kind,
    pub description: i18n::Message,
}

const PARAMETERS: &[Parameter] = &[Parameter {
    name: "--view",
    kind: Kind::Conflict,
    description: i18n::Message::ConflictViewCommand,
}];

pub(crate) struct Query<'a> {
    owner: &'static Command,
    tail: &'a str,
}

fn after_name<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    text.get(..name.len())?
        .eq_ignore_ascii_case(name)
        .then_some(())?;
    text.get(name.len()..)?.strip_prefix(' ')
}

impl Parameter {
    pub fn command(self) -> &'static Command {
        self.kind.command()
    }

    pub fn complete(self, input: &mut Input) {
        input.replace_canonical(Editor::new(format!(
            "/{} {} ",
            self.command().name,
            self.name
        )));
    }
}

impl<'a> Query<'a> {
    pub fn new(input: &'a str) -> Option<Self> {
        let input = input.trim_start();
        PARAMETERS
            .iter()
            .map(|parameter| parameter.command())
            .filter_map(|owner| {
                let tail = if input.eq_ignore_ascii_case(owner.name) {
                    ""
                } else {
                    after_name(input, owner.name)?.trim_start()
                };
                Some(Self { owner, tail })
            })
            .max_by_key(|query| query.owner.name.len())
    }

    pub fn owner(&self) -> &'static Command {
        self.owner
    }

    pub fn parameter_only(&self) -> bool {
        self.tail.starts_with('-')
    }

    pub fn candidates(&self) -> impl Iterator<Item = Parameter> {
        let prefix = self.tail.to_ascii_lowercase();
        parameters(self.owner).filter(move |parameter| parameter.name.starts_with(&prefix))
    }

    pub fn value(&self) -> Option<(Parameter, &'a str)> {
        parameters(self.owner)
            .find_map(|parameter| Some((parameter, after_name(self.tail, parameter.name)?)))
    }
}

pub(crate) fn parameters(command: &Command) -> impl Iterator<Item = Parameter> {
    PARAMETERS
        .iter()
        .copied()
        .filter(|parameter| parameter.command().name == command.name)
}

pub(crate) fn has_parameters(command: &Command) -> bool {
    parameters(command).next().is_some()
}

pub(crate) fn flag_span(input: &str) -> Option<Range<usize>> {
    let query = Query::new(input)?;
    if !query.parameter_only() {
        return None;
    }
    let start = input.len() - query.tail.len();
    let end = query
        .tail
        .find(char::is_whitespace)
        .unwrap_or(query.tail.len());
    Some(start..start + end)
}

#[derive(Clone, Copy)]
pub(crate) enum Candidate {
    Command(&'static Command),
    Parameter(Parameter),
}

impl Candidate {
    pub fn command(self) -> Option<&'static Command> {
        match self {
            Self::Command(command) => Some(command),
            Self::Parameter(_) => None,
        }
    }

    pub fn owner(self) -> &'static Command {
        match self {
            Self::Command(command) => command,
            Self::Parameter(parameter) => parameter.command(),
        }
    }
}
