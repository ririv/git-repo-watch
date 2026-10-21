package check_test

import (
	"context"
	"gw/internal/check"
	"gw/internal/model"
	"gw/internal/testkit"
	"os"
	"path/filepath"
	"testing"
)

func result(t *testing.T, f *testkit.Fixture, policy model.PushPolicy) model.Repository {
	t.Helper()
	return check.Repository(context.Background(), testkit.Runner(), f.Repo, policy).Record()
}
func has(t *testing.T, r model.Repository, kind model.IssueKind) {
	t.Helper()
	if r.Status != model.Attention {
		t.Fatalf("期望需关注，实际：%+v", r)
	}
	for _, i := range r.Issues {
		if i.Kind == kind {
			return
		}
	}
	t.Fatalf("缺少 %s：%+v", kind, r)
}
func TestClean(t *testing.T) {
	f := testkit.New(t)
	if r := result(t, f, model.Disabled); r.Status != model.Clean {
		t.Fatal(r)
	}
}
func TestWorkspaceChanges(t *testing.T) {
	f := testkit.New(t)
	p := filepath.Join(f.Repo, "file.txt")
	testkit.Write(t, p, "one")
	has(t, result(t, f, model.Disabled), model.Dirty)
	testkit.Git(t, f.Repo, "add", "file.txt")
	has(t, result(t, f, model.Disabled), model.Dirty)
	testkit.Git(t, f.Repo, "commit", "-m", "file")
	testkit.Git(t, f.Repo, "push")
	testkit.Write(t, p, "two")
	has(t, result(t, f, model.Disabled), model.Dirty)
}
func TestIgnoredFilesDoNotDirtyWorkspace(t *testing.T) {
	f := testkit.New(t)
	testkit.Write(t, filepath.Join(f.Repo, ".gitignore"), "cache/\n")
	testkit.Git(t, f.Repo, "add", ".gitignore")
	testkit.Git(t, f.Repo, "commit", "-m", "ignore")
	testkit.Git(t, f.Repo, "push")
	testkit.Write(t, filepath.Join(f.Repo, "cache/data"), "cache")
	if r := result(t, f, model.Disabled); r.Status != model.Clean {
		t.Fatal(r)
	}
}
func TestAheadObserveOnly(t *testing.T) {
	f := testkit.New(t)
	f.Commit(t, "ahead")
	before := testkit.Git(t, f.Remote, "rev-parse", "main")
	r := result(t, f, model.Disabled)
	has(t, r, model.OutOfSync)
	if len(r.Pushes) != 0 || testkit.Git(t, f.Remote, "rev-parse", "main") != before {
		t.Fatal("只读检查产生推送")
	}
}
func TestBehindAndDivergedNeverPublish(t *testing.T) {
	f := testkit.New(t)
	f.Advance(t)
	r := result(t, f, model.FastForward)
	has(t, r, model.OutOfSync)
	if len(r.Pushes) != 0 {
		t.Fatal(r)
	}
	f.Commit(t, "local")
	r = result(t, f, model.FastForward)
	has(t, r, model.OutOfSync)
	if len(r.Pushes) != 0 {
		t.Fatal(r)
	}
}
func TestExperimentalBranchesAreSeparate(t *testing.T) {
	f := testkit.New(t)
	testkit.Git(t, f.Repo, "checkout", "-b", "experiment")
	f.Commit(t, "experimental")
	testkit.Git(t, f.Repo, "checkout", "main")
	r := result(t, f, model.FastForward)
	if r.Status != model.Clean || len(r.Branches) != 1 || len(r.Pushes) != 0 {
		t.Fatal(r)
	}
	if got := testkit.Git(t, f.Remote, "for-each-ref", "--format=%(refname)", "refs/heads/"); got != "refs/heads/main" {
		t.Fatal(got)
	}
}
func TestLocalUpstreamIsSeparate(t *testing.T) {
	f := testkit.New(t)
	testkit.Git(t, f.Repo, "branch", "feature")
	testkit.Git(t, f.Repo, "branch", "--set-upstream-to=main", "feature")
	r := result(t, f, model.Disabled)
	if r.Status != model.Clean || r.Branches[0].Reason != model.LocalUpstream {
		t.Fatal(r)
	}
}
func TestDeletedUpstreamNeedsAttention(t *testing.T) {
	f := testkit.New(t)
	testkit.Git(t, f.Repo, "checkout", "-b", "feature")
	testkit.Git(t, f.Repo, "push", "-u", "origin", "feature")
	testkit.Git(t, f.Remote, "update-ref", "-d", "refs/heads/feature")
	has(t, result(t, f, model.FastForward), model.MissingUpstream)
}
func TestDetachedAndUnborn(t *testing.T) {
	f := testkit.New(t)
	testkit.Git(t, f.Repo, "checkout", "--detach")
	has(t, result(t, f, model.Disabled), model.Detached)
	empty := filepath.Join(f.Root, "empty")
	testkit.Init(t, empty)
	r := check.Repository(context.Background(), testkit.Runner(), empty, model.Disabled).Record()
	has(t, r, model.NoCommits)
}
func TestFailedRemoteIsNotClean(t *testing.T) {
	f := testkit.New(t)
	testkit.Git(t, f.Repo, "remote", "set-url", "origin", filepath.Join(f.Root, "missing.git"))
	if r := result(t, f, model.FastForward); r.Status != model.Failed {
		t.Fatal(r)
	}
}
func TestAheadAutoPushWithDirtyWorkspace(t *testing.T) {
	f := testkit.New(t)
	f.Commit(t, "ahead")
	file := filepath.Join(f.Repo, "pending")
	testkit.Write(t, file, "not published")
	r := result(t, f, model.FastForward)
	has(t, r, model.Dirty)
	if len(r.Pushes) != 1 || r.Pushes[0].Status != "pushed" {
		t.Fatal(r)
	}
	if testkit.Git(t, f.Remote, "rev-parse", "main") != testkit.Git(t, f.Repo, "rev-parse", "main") {
		t.Fatal("远程未同步")
	}
	if data, e := os.ReadFile(file); e != nil || string(data) != "not published" {
		t.Fatal(e)
	}
	if testkit.Git(t, f.Remote, "ls-tree", "-r", "--name-only", "main") != "" {
		t.Fatal("发布了未提交文件")
	}
}
func TestLinkedWorktree(t *testing.T) {
	f := testkit.New(t)
	linked := filepath.Join(f.Root, "linked")
	testkit.Git(t, f.Repo, "worktree", "add", "-b", "linked", linked)
	testkit.Git(t, linked, "push", "-u", "origin", "linked")
	r := check.Repository(context.Background(), testkit.Runner(), linked, model.Disabled).Record()
	if r.Status != model.Clean {
		t.Fatal(r)
	}
}
