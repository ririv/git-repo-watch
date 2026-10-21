package app_test

import (
	"context"
	"errors"
	"gw/internal/app"
	"gw/internal/model"
	"gw/internal/scheduler"
	"gw/internal/state"
	"gw/internal/testkit"
	"os"
	"path/filepath"
	"testing"
)

type schedule struct{}

func (schedule) Registration(context.Context) (scheduler.Registration, error) {
	return scheduler.NotLoaded, nil
}
func (schedule) Schedule() (scheduler.Schedule, error) { return scheduler.Missing{}, nil }

type notification struct {
	calls  int
	result string
}

func (n *notification) Failure(_ context.Context, path string, p model.PushRecord) model.Notice {
	n.calls++
	if n.result == "failed" {
		return model.NotificationFailed(path, p.Target.Branch, "通知不可用")
	}
	return model.Accepted(path, p.Target.Branch)
}
func setup(t *testing.T, paths []string, policy model.PushPolicy) *app.Service {
	t.Helper()
	s := app.New(t.TempDir(), schedule{})
	s.Git = testkit.Runner()
	if e := s.Store.WriteConfig(state.Config{Interval: 21600, Repositories: paths, Excluded: []string{}, PushPolicy: policy}); e != nil {
		t.Fatal(e)
	}
	return s
}
func TestPushFailureNotificationAndEvents(t *testing.T) {
	f := testkit.New(t)
	f.Commit(t, "ahead")
	hook := filepath.Join(f.Remote, "hooks/pre-receive")
	testkit.Write(t, hook, "#!/bin/sh\nexit 1\n")
	os.Chmod(hook, 0700)
	s := setup(t, []string{f.Repo}, model.FastForward)
	notice := &notification{result: "failed"}
	s.Notifier = notice
	events := []app.Event{}
	r, e := s.Run(context.Background(), func(event app.Event) { events = append(events, event) })
	if e != nil {
		t.Fatal(e)
	}
	if r.Status != model.Attention || notice.calls != 1 || len(r.Notifications) != 1 || r.Notifications[0].Status != "failed" {
		t.Fatal(r, notice.calls)
	}
	if len(events) != 3 {
		t.Fatal(events)
	}
	saved, e := s.Report()
	if e != nil || saved.Repositories[0].Pushes[0].Status != "failed" {
		t.Fatal(saved, e)
	}
}
func TestEmptyRegistryIsNotClean(t *testing.T) {
	s := setup(t, []string{}, model.Disabled)
	r, e := s.Run(context.Background(), func(app.Event) {})
	if e != nil || r.Status != model.Unconfigured {
		t.Fatal(r, e)
	}
}
func TestRunContentionAndCancellation(t *testing.T) {
	f := testkit.New(t)
	s := setup(t, []string{f.Repo}, model.Disabled)
	lock, e := s.Store.Lock()
	if e != nil {
		t.Fatal(e)
	}
	if _, e = s.Run(context.Background(), func(app.Event) {}); !errors.Is(e, state.ErrRunning) {
		t.Fatal(e)
	}
	state.Unlock(lock)
	ctx, cancel := context.WithCancel(context.Background())
	cancel()
	if _, e = s.Run(ctx, func(app.Event) {}); !errors.Is(e, context.Canceled) {
		t.Fatal(e)
	}
	if _, e = s.Report(); !errors.Is(e, state.ErrNoReport) {
		t.Fatal(e)
	}
}
func TestRegistryBatchIsAtomicAndExclusionPersists(t *testing.T) {
	f := testkit.New(t)
	s := setup(t, []string{}, model.Disabled)
	if _, e := s.Add(context.Background(), []string{f.Repo, filepath.Join(f.Root, "missing")}); e == nil {
		t.Fatal("无效批次不应写入")
	}
	c, _ := s.Config()
	if len(c.Repositories) != 0 {
		t.Fatal(c)
	}
	if _, e := s.Add(context.Background(), []string{f.Repo, f.Repo}); e != nil {
		t.Fatal(e)
	}
	if e := s.Exclude(f.Root); e != nil {
		t.Fatal(e)
	}
	c, _ = s.Config()
	if len(c.Repositories) != 0 || len(c.Excluded) != 1 {
		t.Fatal(c)
	}
	if _, e := s.Register([]string{f.Repo}); e == nil {
		t.Fatal("排除目录不应重新添加")
	}
	if e := s.Unexclude(f.Root); e != nil {
		t.Fatal(e)
	}
	c, _ = s.Config()
	if len(c.Repositories) != 0 {
		t.Fatal("取消排除不应自动导入")
	}
}
