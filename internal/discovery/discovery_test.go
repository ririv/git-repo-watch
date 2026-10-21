package discovery_test

import (
	"context"
	"gw/internal/discovery"
	"gw/internal/testkit"
	"os"
	"path/filepath"
	"testing"
)

func scan(t *testing.T, root string, excluded ...string) discovery.Report {
	t.Helper()
	r, e := discovery.Discover(context.Background(), testkit.Runner(), root, excluded)
	if e != nil {
		t.Fatal(e)
	}
	return r
}
func paths(r discovery.Report) map[string]discovery.Kind {
	out := map[string]discovery.Kind{}
	for _, x := range r.Repositories {
		out[x.Path] = x.Status
	}
	return out
}
func TestNestedIgnoreAndNegation(t *testing.T) {
	testkit.Isolate(t)
	root, _ := filepath.EvalSymlinks(t.TempDir())
	parent := filepath.Join(root, "parent")
	testkit.Init(t, parent)
	for _, dir := range []string{"cache/hidden", "packages/keep", "packages/drop"} {
		testkit.Init(t, filepath.Join(parent, dir))
	}
	testkit.Write(t, filepath.Join(parent, ".gitignore"), "cache/\n")
	testkit.Write(t, filepath.Join(parent, "packages/.gitignore"), "*\n!keep/\n")
	got := paths(scan(t, root))
	if len(got) != 2 || got[parent] != discovery.Scanned || got[filepath.Join(parent, "packages/keep")] != discovery.Scanned {
		t.Fatal(got)
	}
}
func TestGlobalAndRepositoryExclusions(t *testing.T) {
	testkit.Isolate(t)
	root, _ := filepath.EvalSymlinks(t.TempDir())
	parent := filepath.Join(root, "parent")
	testkit.Init(t, parent)
	for _, dir := range []string{"local-hidden", "global-hidden", "explicit-hidden"} {
		testkit.Init(t, filepath.Join(parent, dir))
	}
	testkit.Write(t, filepath.Join(parent, ".git/info/exclude"), "local-hidden/\n")
	global := filepath.Join(root, "global-ignore")
	testkit.Write(t, global, "global-hidden/\n")
	testkit.Git(t, parent, "config", "core.excludesFile", global)
	got := paths(scan(t, root, filepath.Join(parent, "explicit-hidden")))
	if len(got) != 1 || got[parent] != discovery.Scanned {
		t.Fatal(got)
	}
}
func TestNestedRepoOwnRules(t *testing.T) {
	testkit.Isolate(t)
	root, _ := filepath.EvalSymlinks(t.TempDir())
	parent := filepath.Join(root, "parent")
	nested := filepath.Join(parent, "nested")
	testkit.Init(t, parent)
	testkit.Init(t, nested)
	testkit.Init(t, filepath.Join(nested, "cache/hidden"))
	testkit.Init(t, filepath.Join(nested, "visible"))
	testkit.Write(t, filepath.Join(nested, ".gitignore"), "cache/\n")
	if got := paths(scan(t, root)); len(got) != 3 {
		t.Fatal(got)
	}
}
func TestTrackedSubmoduleSurvivesIgnore(t *testing.T) {
	f := testkit.New(t)
	parent := filepath.Join(f.Root, "parent")
	testkit.Init(t, parent)
	testkit.Git(t, parent, "-c", "protocol.file.allow=always", "submodule", "add", f.Remote, "vendor/sub")
	testkit.Write(t, filepath.Join(parent, ".gitignore"), "vendor/\n")
	got := paths(scan(t, parent))
	if len(got) != 2 || got[filepath.Join(parent, "vendor/sub")] != discovery.Scanned {
		t.Fatal(got)
	}
}
func TestTrackedDirectoryDoesNotExposeIgnoredRepo(t *testing.T) {
	f := testkit.New(t)
	testkit.Write(t, filepath.Join(f.Repo, "cache/tracked"), "data")
	testkit.Git(t, f.Repo, "add", "cache/tracked")
	testkit.Write(t, filepath.Join(f.Repo, ".gitignore"), "cache/\n")
	testkit.Init(t, filepath.Join(f.Repo, "cache/hidden"))
	if got := paths(scan(t, f.Repo)); len(got) != 1 {
		t.Fatal(got)
	}
}
func TestWorktreeAndSymlink(t *testing.T) {
	f := testkit.New(t)
	linked := filepath.Join(f.Root, "linked")
	testkit.Git(t, f.Repo, "worktree", "add", "-b", "linked", linked)
	if e := os.Symlink(f.Repo, filepath.Join(f.Root, "alias")); e != nil {
		t.Fatal(e)
	}
	got := paths(scan(t, f.Root))
	if len(got) != 2 || got[linked] != discovery.Scanned {
		t.Fatal(got)
	}
}
func TestBinarySafePathNames(t *testing.T) {
	f := testkit.New(t)
	nested := filepath.Join(f.Repo, "中文\n目录\t/nested")
	testkit.Init(t, nested)
	got := paths(scan(t, f.Repo))
	if len(got) != 2 || got[nested] != discovery.Scanned {
		t.Fatal(got)
	}
}
func TestBrokenWorktreeStopsBlindTraversal(t *testing.T) {
	testkit.Isolate(t)
	root, _ := filepath.EvalSymlinks(t.TempDir())
	broken := filepath.Join(root, "broken")
	testkit.Write(t, filepath.Join(broken, ".git"), "gitdir: /missing/repository")
	testkit.Init(t, filepath.Join(broken, "do-not-visit"))
	got := scan(t, root)
	if len(got.Repositories) != 1 || got.Repositories[0].Status != discovery.Unreadable {
		t.Fatal(got)
	}
}
func TestEmptyMarkerIsNotRepository(t *testing.T) {
	testkit.Isolate(t)
	root, _ := filepath.EvalSymlinks(t.TempDir())
	testkit.Write(t, filepath.Join(root, "cache/.git"), "")
	r := scan(t, root)
	if len(r.Repositories) != 0 || len(r.Markers) != 1 {
		t.Fatal(r)
	}
}
func TestSubdirectoryInheritsAndExplicitRootOverrides(t *testing.T) {
	f := testkit.New(t)
	hidden := filepath.Join(f.Repo, "packages/hidden")
	visible := filepath.Join(f.Repo, "packages/visible")
	testkit.Init(t, hidden)
	testkit.Init(t, visible)
	testkit.Write(t, filepath.Join(f.Repo, ".gitignore"), "packages/hidden/\n")
	if got := paths(scan(t, filepath.Join(f.Repo, "packages"))); len(got) != 1 || got[visible] != discovery.Scanned {
		t.Fatal(got)
	}
	if got := paths(scan(t, hidden)); len(got) != 1 || got[hidden] != discovery.Scanned {
		t.Fatal(got)
	}
}
func TestMissingRootErrors(t *testing.T) {
	_, e := discovery.Discover(context.Background(), testkit.Runner(), filepath.Join(t.TempDir(), "missing"), nil)
	if e == nil {
		t.Fatal("缺失目录不应当作空扫描")
	}
}
