//! A partial plan can only be minted by target preparation and consumed once.
use crate::{Effect, Mode, Outcome, Screen};
use engine::local::files::partial;

#[derive(Debug)]
pub(crate) struct PreparedPartial {
    plan: partial::Plan,
}
impl PreparedPartial {
    pub(in crate::command::target) fn new(plan: partial::Plan) -> Self {
        Self { plan }
    }
    pub(in crate::command::target) fn review(self, screen: &mut Screen) {
        if self.plan.action() != partial::Action::Discard {
            self.execute(screen);
            return;
        }
        let review = match self.plan.preview() {
            Ok(patch) => format!(
                "{}\n\n{}\n\n{patch}",
                i18n::tr!(PartialDiscardConfirm),
                self.plan.snapshot().change().location.display()
            ),
            Err(error) => {
                screen.message = crate::message::Message::error(format!("{error:#}"));
                return;
            }
        };
        let origin = Box::new(screen.take_mode());
        screen.mode = Mode::Confirm {
            operation: crate::Confirmation::Partial {
                plan: Box::new(self),
                review,
                origin,
            },
            choice: crate::confirmation::Choice::Return,
        };
    }
    pub(crate) fn execute(self, screen: &mut Screen) {
        let plan = self.plan;
        let (title, effect) = match plan.action() {
            partial::Action::Stage => (i18n::tr!(PartialStage), Effect::IndexWrite),
            partial::Action::Unstage => (i18n::tr!(PartialUnstage), Effect::IndexWrite),
            partial::Action::Discard => (i18n::tr!(PartialDiscard), Effect::RepositoryWrite),
        };
        screen.start_repository(
            plan.snapshot().files().path.clone(),
            title,
            effect,
            move |service, cancel, _| Ok(Outcome::Files(service.edit_partial(&plan, &cancel)?)),
        );
    }
}
