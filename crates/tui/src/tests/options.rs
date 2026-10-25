use super::*;
use crate::{
    command::{
        completion::Menu,
        options::{Kind, Value},
    },
    repository_list::{Filter, Order},
};

fn choices(s: &Screen) -> &command::options::Candidates {
    let bar::InputBar::Command {
        menu: Menu::Options(values),
        ..
    } = &s.bar
    else {
        panic!("parameter options expected")
    };
    values
}

#[test]
fn fixed_options_have_one_command_and_automatic_localized_parameter_candidates() {
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec!["/example/a".into()]);
        for (name, kind, count, argument) in [
            ("repos filter", Kind::Filter, 7, "[all|problems|…]"),
            ("repos sort", Kind::Sort, 4, "[priority|name|…]"),
            ("repo diff view", Kind::FileView, 2, "[tree|flat]"),
            (
                "repo diff layout",
                Kind::DiffLayout,
                3,
                "[auto|split|unified]",
            ),
            ("search", Kind::Search, 2, "[filter|locate]"),
            (
                "settings interaction",
                Kind::Interaction,
                2,
                "[interactive|text]",
            ),
        ] {
            assert_eq!(command::matches(name).len(), 1);
            assert_eq!(kind.command().usage(), format!("{name} {argument}"));
            s.paste(&format!("//{name} "));
            assert_eq!(choices(&s).options.len(), count);
            assert_eq!(s.search_commands().unwrap().len(), 1);
            for (width, height) in [(60, 24), (120, 30), (1, 1), (0, 0)] {
                render(&mut s, width, height);
            }
            key(&mut s, KeyCode::Esc);
            key(&mut s, KeyCode::Esc);
        }
        s.paste("//repos filter ");
        assert_eq!(
            choices(&s).options[choices(&s).selected].value,
            Value::Filter(Filter::All)
        );
        let text = render(&mut s, 120, 30);
        assert_eq!(text.matches("/repos filter").count(), 1);
        assert!(text.contains(i18n::tr!(RepositoryFilterFailed)));
        for width in [60, 120] {
            let text = render(&mut s, width, 30);
            let current = text.lines().find(|line| line.contains("│all ")).unwrap();
            assert!(current.ends_with("✓│"), "{current}");
        }
        key(&mut s, KeyCode::Down);
        key(&mut s, KeyCode::Tab);
        assert!(s.bar.text().ends_with("filter problems"));
        assert_eq!(s.filter, Filter::All);
        key(&mut s, KeyCode::Enter);
        assert_eq!(s.filter, Filter::Problems);
        command(&mut s, "repos filter failed");
        assert_eq!(s.filter, Filter::Failed);
        command(&mut s, "repos filter 未配置远程上游");
        assert_eq!(s.filter, Filter::WithoutUpstream);
        command(&mut s, "repos filter invalid-value");
        assert_eq!(s.filter, Filter::WithoutUpstream);
        assert!(choices(&s).options.is_empty());
        assert_eq!(s.message, i18n::tr!(NoMatchingOptions));
        command(&mut s, "repos filter all");
        command(&mut s, "repos sort n");
        assert_eq!(s.order, Order::Name);
        s.shutdown();
    }
}

#[test]
fn value_completion_preserves_plain_search_and_the_selected_repository_until_execution() {
    let (_home, mut s) = fixture(vec!["/example/中文-a".into(), "/example/中文-b".into()]);
    s.paste("中文");
    let selected = s.selected_path();
    let config = std::fs::read(s.service.store.config_path()).unwrap();
    s.paste("//repos sort ");
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Tab);
    assert_eq!(s.order, Order::Priority);
    assert_eq!(s.selected_path(), selected);
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.order, Order::Name);
    assert_eq!(s.bar.text(), "中文");
    assert_eq!(s.selected_path(), selected);
    assert_eq!(
        std::fs::read(s.service.store.config_path()).unwrap(),
        config
    );
    s.shutdown();
}
