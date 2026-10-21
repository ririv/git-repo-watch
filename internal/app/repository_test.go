package app_test

import (
	"context"
	"errors"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"gw/internal/model"
	"gw/internal/state"
	"gw/internal/testkit"
)

func TestManualCommitAndPushKeepPeriodicReportUnchanged(t *testing.T) {
	f := testkit.New(t)
	s := setup(t, []string{f.Repo}, model.Disabled)
	reportPath := filepath.Join(s.Store.StateDir(), "latest.json")
	lock, e := s.Store.Lock()
	if e != nil {
		t.Fatal(e)
	}
	state.Unlock(lock)
	before := []byte("cached periodic report\n")
	testkit.Write(t, reportPath, string(before))
	testkit.Write(t, filepath.Join(f.Repo, "file"), "staged")
	testkit.Git(t, f.Repo, "add", "file")
	p, e := s.PrepareCommit(context.Background(), f.Repo)
	if e != nil {
		t.Fatal(e)
	}
	committed, e := s.CommitRepository(context.Background(), p, "manual commit")
	if e != nil {
		t.Fatal(e)
	}
	push, e := s.PreparePush(context.Background(), f.Repo)
	if e != nil {
		t.Fatal(e)
	}
	result, e := s.PushRepository(context.Background(), push)
	if e != nil || result.Record().Status != "pushed" {
		t.Fatal(result, e)
	}
	if testkit.Git(t, f.Remote, "rev-parse", "main") != committed.OID {
		t.Fatal("没有推送当前分支")
	}
	after, e := os.ReadFile(reportPath)
	if e != nil || string(after) != string(before) {
		t.Fatal(string(after), e)
	}
	for _, name := range []string{"manual-commit.json", "manual-push.json"} {
		info, e := os.Stat(filepath.Join(s.Store.StateDir(), name))
		if e != nil || info.Mode().Perm() != 0600 {
			t.Fatal(name, info, e)
		}
	}
}
func TestManualOperationRequiresCurrentRegistrationAndSharedLock(t *testing.T) {
	f := testkit.New(t)
	s := setup(t, []string{f.Repo}, model.Disabled)
	lock, e := s.Store.Lock()
	if e != nil {
		t.Fatal(e)
	}
	_, e = s.PrepareCommit(context.Background(), f.Repo)
	if !errors.Is(e, state.ErrRunning) {
		t.Fatal(e)
	}
	state.Unlock(lock)
	testkit.Write(t, filepath.Join(f.Repo, "file"), "staged")
	testkit.Git(t, f.Repo, "add", "file")
	p, e := s.PrepareCommit(context.Background(), f.Repo)
	if e != nil {
		t.Fatal(e)
	}
	before := testkit.Git(t, f.Repo, "rev-parse", "HEAD")
	if e = s.Remove(f.Repo); e != nil {
		t.Fatal(e)
	}
	if _, e = s.CommitRepository(context.Background(), p, "removed"); e == nil || !strings.Contains(e.Error(), "注册列表") {
		t.Fatal(e)
	}
	if testkit.Git(t, f.Repo, "rev-parse", "HEAD") != before {
		t.Fatal("已移除的仓库不应继续执行操作")
	}
}
