package repoaction_test

import (
	"context"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"gw/internal/model"
	"gw/internal/repoaction"
	"gw/internal/testkit"
)

func staged(t *testing.T, f *testkit.Fixture) repoaction.CommitPlan {
	t.Helper()
	testkit.Write(t, filepath.Join(f.Repo, "file.txt"), "staged\n")
	testkit.Git(t, f.Repo, "add", "file.txt")
	p, e := repoaction.PrepareCommit(context.Background(), testkit.Runner(), f.Repo)
	if e != nil {
		t.Fatal(e)
	}
	return p
}
func TestCommitOnlyUsesReviewedIndexAndKeepsWorkingFiles(t *testing.T) {
	f := testkit.New(t)
	p := staged(t, f)
	testkit.Write(t, filepath.Join(f.Repo, "file.txt"), "unstaged\n")
	testkit.Write(t, filepath.Join(f.Repo, "untracked.txt"), "private\n")
	r, e := repoaction.Commit(context.Background(), testkit.Runner(), p, "提交暂存内容")
	if e != nil || r.Review != repoaction.ExactCommit {
		t.Fatal(r, e)
	}
	if got := testkit.Git(t, f.Repo, "show", "HEAD:file.txt"); got != "staged" {
		t.Fatal(got)
	}
	files := testkit.Git(t, f.Repo, "ls-tree", "--name-only", "HEAD")
	if strings.Contains(files, "untracked") {
		t.Fatal(files)
	}
	data, _ := os.ReadFile(filepath.Join(f.Repo, "file.txt"))
	if string(data) != "unstaged\n" {
		t.Fatal(string(data))
	}
	if len(p.Files()) != 1 || p.Branch() != "main" {
		t.Fatal(p.Files(), p.Branch())
	}
}
func TestChangedIndexOrBranchRequiresNewReview(t *testing.T) {
	for _, change := range []string{"index", "branch"} {
		t.Run(change, func(t *testing.T) {
			f := testkit.New(t)
			p := staged(t, f)
			before := testkit.Git(t, f.Repo, "rev-parse", "HEAD")
			if change == "index" {
				testkit.Write(t, filepath.Join(f.Repo, "file.txt"), "new\n")
				testkit.Git(t, f.Repo, "add", "file.txt")
			} else {
				testkit.Git(t, f.Repo, "switch", "-c", "another")
			}
			if _, e := repoaction.Commit(context.Background(), testkit.Runner(), p, "should not commit"); e == nil {
				t.Fatal("修改后应重新确认")
			}
			if now := testkit.Git(t, f.Repo, "rev-parse", "HEAD"); now != before {
				t.Fatal(now, before)
			}
		})
	}
}
func TestNoStagedChangesAndRejectingCommitHook(t *testing.T) {
	f := testkit.New(t)
	if _, e := repoaction.PrepareCommit(context.Background(), testkit.Runner(), f.Repo); e == nil || !strings.Contains(e.Error(), "没有已暂存") {
		t.Fatal(e)
	}
	p := staged(t, f)
	before := testkit.Git(t, f.Repo, "rev-parse", "HEAD")
	hook := filepath.Join(f.Repo, ".git/hooks/pre-commit")
	testkit.Write(t, hook, "#!/bin/sh\nexit 1\n")
	os.Chmod(hook, 0700)
	if _, e := repoaction.Commit(context.Background(), testkit.Runner(), p, "rejected"); e == nil {
		t.Fatal("必须保留提交钩子")
	}
	if testkit.Git(t, f.Repo, "rev-parse", "HEAD") != before {
		t.Fatal("钩子拒绝后不应生成提交")
	}
}
func TestCommitHookChangesAreReportedAfterSuccessfulCommit(t *testing.T) {
	f := testkit.New(t)
	p := staged(t, f)
	hook := filepath.Join(f.Repo, ".git/hooks/pre-commit")
	testkit.Write(t, hook, "#!/bin/sh\nprintf 'hook' > hook.txt\ngit add hook.txt\n")
	os.Chmod(hook, 0700)
	r, e := repoaction.Commit(context.Background(), testkit.Runner(), p, "hook changes")
	if e != nil || r.Review != repoaction.ChangedCommit || r.OID == "" {
		t.Fatal(r, e)
	}
}
func TestInitialCommitIsSupported(t *testing.T) {
	testkit.Isolate(t)
	path := filepath.Join(t.TempDir(), "initial")
	testkit.Init(t, path)
	testkit.Write(t, filepath.Join(path, "first"), "first")
	testkit.Git(t, path, "add", "first")
	p, e := repoaction.PrepareCommit(context.Background(), testkit.Runner(), path)
	if e != nil {
		t.Fatal(e)
	}
	r, e := repoaction.Commit(context.Background(), testkit.Runner(), p, "initial")
	if e != nil || r.Review != repoaction.ExactCommit {
		t.Fatal(r, e)
	}
}
func TestPushOnlyPublishesCurrentBranch(t *testing.T) {
	f := testkit.New(t)
	f.Commit(t, "main ahead")
	testkit.Git(t, f.Repo, "switch", "-c", "experiment")
	f.Commit(t, "experiment ahead")
	testkit.Git(t, f.Repo, "switch", "main")
	testkit.Write(t, filepath.Join(f.Repo, "untracked"), "keep")
	p, e := repoaction.PreparePush(context.Background(), testkit.Runner(), f.Repo)
	if e != nil {
		t.Fatal(e)
	}
	if p.Target().Ahead != 1 || p.Target().Branch != "main" {
		t.Fatal(p.Target())
	}
	result := repoaction.Push(context.Background(), testkit.Runner(), p).Record()
	if result.Status != "pushed" {
		t.Fatal(result)
	}
	if testkit.Git(t, f.Remote, "rev-parse", "main") != p.Target().LocalOID {
		t.Fatal("没有发布选中的快照")
	}
	if branches := testkit.Git(t, f.Remote, "for-each-ref", "--format=%(refname)", "refs/heads/"); strings.Contains(branches, "experiment") {
		t.Fatal(branches)
	}
	if _, e = os.Stat(filepath.Join(f.Repo, "untracked")); e != nil {
		t.Fatal(e)
	}
}
func TestPushReviewRejectsChangedRemoteOrCurrentBranch(t *testing.T) {
	for _, change := range []string{"remote", "branch", "url"} {
		t.Run(change, func(t *testing.T) {
			f := testkit.New(t)
			f.Commit(t, "ahead")
			p, e := repoaction.PreparePush(context.Background(), testkit.Runner(), f.Repo)
			if e != nil {
				t.Fatal(e)
			}
			switch change {
			case "remote":
				f.Advance(t)
			case "branch":
				testkit.Git(t, f.Repo, "switch", "-c", "another")
			case "url":
				testkit.Git(t, f.Repo, "remote", "set-url", "origin", filepath.Join(f.Root, "other.git"))
			}
			before := testkit.Git(t, f.Remote, "rev-parse", "main")
			r := repoaction.Push(context.Background(), testkit.Runner(), p).Record()
			if r.Status != "failed" || r.Phase != model.Validate {
				t.Fatal(r)
			}
			if testkit.Git(t, f.Remote, "rev-parse", "main") != before {
				t.Fatal("确认内容变化后不应推送")
			}
		})
	}
}
func TestPushPreparationExplainsMissingUpstreamOrRemoteChanges(t *testing.T) {
	for _, scenario := range []string{"same", "no-upstream", "behind", "diverged", "local-upstream"} {
		t.Run(scenario, func(t *testing.T) {
			f := testkit.New(t)
			switch scenario {
			case "no-upstream":
				testkit.Git(t, f.Repo, "branch", "--unset-upstream")
			case "behind":
				f.Advance(t)
			case "diverged":
				f.Advance(t)
				f.Commit(t, "local ahead")
			case "local-upstream":
				testkit.Git(t, f.Repo, "branch", "local-base")
				testkit.Git(t, f.Repo, "branch", "--set-upstream-to=local-base")
			}
			_, e := repoaction.PreparePush(context.Background(), testkit.Runner(), f.Repo)
			if e == nil {
				t.Fatal("应说明无法直接推送的原因")
			}
		})
	}
}
