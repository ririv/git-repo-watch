//! Validated repository workflows with explicit action order and mutation scopes.
mod run;
use crate::domain::PushPolicy;
use anyhow::{Result, ensure};
pub(crate) use run::execute;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Step {
    Fetch,
    Pull,
    Push,
    Inspect,
}
impl Step {
    pub const EDITABLE: [Self; 3] = [Self::Fetch, Self::Pull, Self::Push];
    pub fn token(self) -> &'static str {
        match self {
            Self::Fetch => "fetch",
            Self::Pull => "pull",
            Self::Push => "push",
            Self::Inspect => "inspect",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Fetch => i18n::tr!(WorkflowFetch),
            Self::Pull => i18n::tr!(WorkflowPull),
            Self::Push => i18n::tr!(WorkflowPush),
            Self::Inspect => i18n::tr!(WorkflowInspect),
        }
    }
    pub fn description(self) -> &'static str {
        match self {
            Self::Fetch => i18n::tr!(WorkflowFetchDescription),
            Self::Pull => i18n::tr!(WorkflowPullDescription),
            Self::Push => i18n::tr!(WorkflowPushDescription),
            Self::Inspect => i18n::tr!(WorkflowInspectDescription),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    Startup,
    Scheduled,
}
impl Trigger {
    pub fn label(self) -> &'static str {
        match self {
            Self::Startup => i18n::tr!(WorkflowStartup),
            Self::Scheduled => i18n::tr!(WorkflowScheduled),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Vec<Step>", into = "Vec<Step>")]
pub struct Plan {
    steps: Vec<Step>,
}
impl Default for Plan {
    fn default() -> Self {
        Self::fetch()
    }
}
impl Plan {
    pub fn local() -> Self {
        Self {
            steps: vec![Step::Inspect],
        }
    }
    pub fn fetch() -> Self {
        Self {
            steps: vec![Step::Fetch, Step::Inspect],
        }
    }
    pub fn from_policy(policy: PushPolicy) -> Self {
        match policy {
            PushPolicy::Disabled => Self::fetch(),
            PushPolicy::FastForward => Self {
                steps: vec![Step::Fetch, Step::Push, Step::Inspect],
            },
        }
    }
    pub fn steps(&self) -> &[Step] {
        &self.steps
    }
    pub fn push_policy(&self) -> PushPolicy {
        if self.steps.contains(&Step::Push) {
            PushPolicy::FastForward
        } else {
            PushPolicy::Disabled
        }
    }
    pub fn tokens(&self) -> String {
        self.steps
            .iter()
            .map(|step| step.token())
            .collect::<Vec<_>>()
            .join(" ")
    }
    pub fn summary(&self) -> String {
        self.steps
            .iter()
            .map(|step| step.label())
            .collect::<Vec<_>>()
            .join(" → ")
    }
    pub fn details(&self) -> String {
        self.steps
            .iter()
            .enumerate()
            .map(|(index, step)| {
                format!("{}. {}\n   {}", index + 1, step.label(), step.description())
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}
impl TryFrom<Vec<Step>> for Plan {
    type Error = anyhow::Error;
    fn try_from(steps: Vec<Step>) -> Result<Self> {
        ensure!(
            steps.last() == Some(&Step::Inspect),
            i18n::tr!(WorkflowInspectLast)
        );
        for (index, step) in steps.iter().enumerate() {
            ensure!(
                !steps[..index].contains(step),
                i18n::tr!(WorkflowDuplicateStep)
            );
            if *step == Step::Push {
                ensure!(
                    steps[..index]
                        .iter()
                        .any(|step| matches!(step, Step::Fetch | Step::Pull)),
                    i18n::tr!(WorkflowPushNeedsRefresh)
                );
            }
        }
        Ok(Self { steps })
    }
}
impl From<Plan> for Vec<Step> {
    fn from(plan: Plan) -> Self {
        plan.steps
    }
}
impl std::str::FromStr for Plan {
    type Err = anyhow::Error;
    fn from_str(text: &str) -> Result<Self> {
        text.split_whitespace()
            .map(|token| match token {
                "fetch" => Ok(Step::Fetch),
                "pull" => Ok(Step::Pull),
                "push" => Ok(Step::Push),
                "inspect" => Ok(Step::Inspect),
                _ => anyhow::bail!(i18n::tf!(WorkflowUnknownStep, token)),
            })
            .collect::<Result<Vec<_>>>()?
            .try_into()
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepRecord {
    pub step: Step,
    pub started_at: String,
    pub finished_at: String,
    pub outcome: StepOutcome,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum StepOutcome {
    Succeeded { detail: String },
    Failed { error: String },
}
