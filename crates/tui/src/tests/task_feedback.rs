use super::*;
use crate::loading::{FEEDBACK_DELAY, Feedback};
use std::time::Instant;

fn pending(s: &mut Screen) -> &mut Pending {
    let Mode::Busy(pending) = &mut s.mode else {
        panic!("foreground task is pending")
    };
    pending
}
fn started(s: &mut Screen) -> Instant {
    let Feedback::Quiet { since, .. } = &pending(s).feedback else {
        panic!("task starts quietly")
    };
    *since
}
fn controlled(s: &mut Screen) -> mpsc::Sender<()> {
    let (release, gate) = mpsc::channel();
    s.start("controlled task", Effect::ReadOnly, move |_, cancel, _| {
        gate.recv()?;
        cancel.check()?;
        Ok(Outcome::Text("finished".into()))
    });
    release
}
fn footer(s: &mut Screen, width: u16, height: u16) -> Vec<String> {
    let text = render(s, width, height);
    let input = s.input_area.expect("bordered input");
    text.lines()
        .skip(input.y.into())
        .take(input.height as usize + 1)
        .map(str::to_owned)
        .collect()
}

#[test]
fn fast_ai_toggle_retains_input_presentation_and_applies_without_waiting_for_feedback() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        command(&mut s, "settings");
        let items = Configuration::General.items(s.snapshot.registration, &s.config.ai);
        s.setting_selected = items
            .iter()
            .position(|item| *item == crate::settings::Item::AiEnabled)
            .unwrap();
        let before = footer(&mut s, 100, 32);
        let enabled = s.config.ai.enabled;
        s.activate_setting(crate::settings::Item::AiEnabled, &Cancel::default())
            .unwrap();
        let now = started(&mut s);
        assert_eq!(footer(&mut s, 100, 32), before);
        assert!(!pending(&mut s).tick(now + FEEDBACK_DELAY - Duration::from_millis(1)));
        assert_eq!(footer(&mut s, 100, 32), before);
        wait(&mut s);
        assert_eq!(s.config.ai.enabled, !enabled);
        assert_eq!(s.service.store.config().unwrap().ai.enabled, !enabled);
        assert!(
            !footer(&mut s, 100, 32)
                .join("\n")
                .contains(i18n::tr!(RunningAlt))
        );
        s.shutdown();
    }
}

#[test]
fn long_task_reveals_running_input_at_the_deadline_and_still_blocks_editing() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec!["/example/alpha".into()]);
        s.paste("alpha");
        let before = footer(&mut s, 100, 32);
        let release = controlled(&mut s);
        let now = started(&mut s);
        assert_eq!(footer(&mut s, 100, 32), before);
        key(&mut s, KeyCode::Char('x'));
        assert_eq!(footer(&mut s, 100, 32), before);
        assert!(!pending(&mut s).tick(now + FEEDBACK_DELAY - Duration::from_millis(1)));
        assert!(pending(&mut s).tick(now + FEEDBACK_DELAY));
        let visible = footer(&mut s, 100, 32).join("\n");
        assert!(visible.contains(i18n::tr!(RunningAlt)), "{visible}");
        assert!(visible.contains("controlled task"), "{visible}");
        release.send(()).unwrap();
        wait(&mut s);
        assert!(!matches!(s.mode, Mode::Busy(_)));
        s.shutdown();
    }
}

#[test]
fn cancellation_and_failure_are_visible_without_waiting_for_running_feedback() {
    let (_home, mut s) = fixture(vec![]);
    command(&mut s, "settings");
    let release = controlled(&mut s);
    key(&mut s, KeyCode::Esc);
    assert_eq!(pending(&mut s).state, TaskState::Cancelling);
    assert!(
        footer(&mut s, 100, 32)
            .join("\n")
            .contains(i18n::tr!(Cancelling))
    );
    release.send(()).unwrap();
    wait(&mut s);
    assert!(matches!(s.mode, Mode::Error { .. }));
    key(&mut s, KeyCode::Esc);
    s.start("fast failure", Effect::ReadOnly, |_, _, _| {
        anyhow::bail!("failure remains visible")
    });
    wait(&mut s);
    let text = render(&mut s, 100, 32);
    assert!(text.contains("failure remains visible"), "{text}");
    s.shutdown();
}
