use super::*;
use crate::remote_info::Content;

fn ready(s: &mut Screen) {
    let start = Instant::now();
    loop {
        s.poll_remote_info();
        if matches!(s.remote_info.content, Content::Ready { .. }) {
            break;
        }
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "remote reader did not finish"
        );
        thread::sleep(Duration::from_millis(3));
    }
}

#[test]
fn remote_links_show_in_both_details_without_replacing_the_local_folder_name() {
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        let url = "https://example.com/team/actual-project.git";
        support::git(&f.repo, &["remote", "set-url", "origin", url]);
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        ready(&mut s);
        let text = render(&mut s, 180, 45);
        assert!(text.contains(url), "{text}");
        assert!(text.contains(i18n::tr!(RemoteRepositories)), "{text}");
        assert!(
            s.pointer
                .links
                .cells
                .values()
                .any(|target| target.uri() == url)
        );
        assert_eq!(s.selected_path().as_deref(), Some(f.repo.as_path()));
        key(&mut s, KeyCode::Enter);
        super::overview::wait_overview(&mut s);
        wait_workspace(&mut s);
        ready(&mut s);
        s.workspace.session_mut().unwrap().selection = crate::workspace::Section::Overview;
        key(&mut s, KeyCode::Enter);
        command(&mut s, "zoom");
        let text = render(&mut s, 160, 45);
        assert_eq!(text.matches(url).count(), 1, "{text}");
        assert!(
            s.pointer
                .links
                .cells
                .values()
                .any(|target| target.uri() == url)
        );
        s.shutdown();
    }
}

#[test]
fn selection_changes_discard_old_results_and_manual_refresh_rereads_local_config() {
    let first = support::Fixture::create();
    let second = support::Fixture::create();
    let first_url = "https://example.com/team/first.git";
    let second_url = "https://example.com/team/second.git";
    support::git(&first.repo, &["remote", "set-url", "origin", first_url]);
    support::git(&second.repo, &["remote", "set-url", "origin", second_url]);
    let (_home, mut s) = fixture(vec![first.repo.clone(), second.repo.clone()]);
    let choose = |s: &mut Screen, path: &PathBuf| {
        let index = s.rows().iter().position(|row| &row.path == path).unwrap();
        s.select(index);
    };
    choose(&mut s, &first.repo);
    s.poll_remote_info();
    choose(&mut s, &second.repo);
    ready(&mut s);
    let text = render(&mut s, 180, 40);
    assert!(text.contains(second_url), "{text}");
    assert!(!text.contains(first_url), "{text}");
    support::git(&second.repo, &["remote", "remove", "origin"]);
    s.refresh_current_view(&Cancel::default());
    ready(&mut s);
    let text = render(&mut s, 180, 40);
    assert!(text.contains(i18n::tr!(RemoteNone)), "{text}");
    assert!(!text.contains(second_url), "{text}");
    s.shutdown();
}
