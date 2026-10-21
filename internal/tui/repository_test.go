package tui

import (
	"context"
	"path/filepath"
	"strings"
	"testing"

	tea "charm.land/bubbletea/v2"
	"charm.land/lipgloss/v2"
	"gw/internal/app"
	"gw/internal/model"
	"gw/internal/state"
	"gw/internal/testkit"
)

func repositoryScreen(t *testing.T) (*screen, *testkit.Fixture) {
	t.Helper()
	f := testkit.New(t)
	s := app.New(t.TempDir(), scheduleReader{})
	s.Git = testkit.Runner()
	if e := s.Store.WriteConfig(state.Config{Interval: 21600, Repositories: []string{f.Repo}, Excluded: []string{}, PushPolicy: model.Disabled}); e != nil {
		t.Fatal(e)
	}
	m := newScreen(context.Background(), s, func(context.Context) error { t.Error("单仓库操作不应安装系统任务"); return nil })
	m.dispatch = func(tea.Msg) {}
	m.Update(m.Init()())
	m.Update(tea.WindowSizeMsg{Width: 100, Height: 28})
	return m, f
}
func TestCommitWorkflowPreviewsBeforeWritingAndCanCancel(t *testing.T) {
	m, f := repositoryScreen(t)
	testkit.Write(t, filepath.Join(f.Repo, "staged.txt"), "staged")
	testkit.Git(t, f.Repo, "add", "staged.txt")
	before := testkit.Git(t, f.Repo, "rev-parse", "HEAD")
	cmd := press(m, "C")
	m.Update(cmd())
	if _, ok := m.mode.(commitEditing); !ok {
		t.Fatal(m.mode, m.message)
	}
	view := m.View().Content
	if !strings.Contains(view, "staged.txt") || !strings.Contains(view, "仅已暂存内容") || strings.Contains(view, "上次检查") {
		t.Fatal(view)
	}
	press(m, "enter")
	if _, ok := m.mode.(commitEditing); !ok {
		t.Fatal(m.mode)
	}
	press(m, "esc")
	if testkit.Git(t, f.Repo, "rev-parse", "HEAD") != before {
		t.Fatal("取消不能提交")
	}
	cmd = press(m, "C")
	m.Update(cmd())
	for _, r := range "manual commit" {
		m.Update(tea.KeyPressMsg{Code: r, Text: string(r)})
	}
	press(m, "enter")
	if _, ok := m.mode.(commitConfirming); !ok {
		t.Fatal(m.mode)
	}
	if !strings.Contains(m.repositoryContent(), "manual commit") {
		t.Fatal(m.repositoryContent())
	}
	if testkit.Git(t, f.Repo, "rev-parse", "HEAD") != before {
		t.Fatal("确认前不应提交")
	}
	settle(m, press(m, "y"))
	m.workers.Wait()
	if testkit.Git(t, f.Repo, "log", "-1", "--format=%s") != "manual commit" || !strings.Contains(m.message, "已提交") {
		t.Fatal(m.message)
	}
}
func (m *screen) repositoryContent() string { content, _ := m.repositoryReview(); return content }

func TestPushWorkflowPublishesOnlyAfterConfirmation(t *testing.T) {
	m, f := repositoryScreen(t)
	f.Commit(t, "manual ahead")
	before := testkit.Git(t, f.Remote, "rev-parse", "main")
	cmd := press(m, "P")
	m.Update(cmd())
	if _, ok := m.mode.(pushConfirming); !ok {
		t.Fatal(m.mode, m.message)
	}
	view := m.View().Content
	if !strings.Contains(view, "refs/heads/main") || !strings.Contains(view, "待推送：1 个提交") {
		t.Fatal(view)
	}
	press(m, "n")
	if testkit.Git(t, f.Remote, "rev-parse", "main") != before {
		t.Fatal("取消不能推送")
	}
	cmd = press(m, "P")
	m.Update(cmd())
	settle(m, press(m, "y"))
	m.workers.Wait()
	if testkit.Git(t, f.Remote, "rev-parse", "main") == before || !strings.Contains(m.message, "已推送") {
		t.Fatal(m.message)
	}
}
func TestRepositoryFailureShowsFullScrollableError(t *testing.T) {
	m, _ := repositoryScreen(t)
	cmd := press(m, "C")
	m.Update(cmd())
	if _, ok := m.mode.(repositoryError); !ok {
		t.Fatal(m.mode, m.message)
	}
	if !strings.Contains(m.View().Content, "没有已暂存的改动") {
		t.Fatal(m.View().Content)
	}
	press(m, "enter")
	if _, ok := m.mode.(browsing); !ok {
		t.Fatal(m.mode)
	}
}
func TestRepositoryReviewFitsNarrowAndWideTerminals(t *testing.T) {
	m, f := repositoryScreen(t)
	testkit.Write(t, filepath.Join(f.Repo, "file"), "staged")
	testkit.Git(t, f.Repo, "add", "file")
	cmd := press(m, "C")
	m.Update(cmd())
	editing := m.mode.(commitEditing)
	f.Commit(t, "for push")
	push, e := m.backend.PreparePush(context.Background(), f.Repo)
	if e != nil {
		t.Fatal(e)
	}
	for _, size := range []tea.WindowSizeMsg{{Width: 60, Height: 18}, {Width: 80, Height: 24}, {Width: 150, Height: 32}} {
		for _, mode := range []interaction{editing, commitConfirming{editing.plan, "reviewed message"}, pushConfirming{push}, repositoryError{commitSelected, "hook failed"}} {
			m.mode = mode
			m.Update(size)
			view := m.View().Content
			if lipgloss.Width(view) > size.Width || lipgloss.Height(view) > size.Height {
				t.Fatalf("size %+v mode %T produced %dx%d", size, mode, lipgloss.Width(view), lipgloss.Height(view))
			}
		}
	}
}
func TestPositionCounterIsAbsentFromListAndOperationMenu(t *testing.T) {
	m, _ := fixture(t)
	for _, view := range []string{m.listView(), m.menuView(0)} {
		if strings.Contains(view, "第 ") {
			t.Fatal(view)
		}
	}
}

func TestCancelDuringPreparationDiffersFromInterruptedMutation(t *testing.T) {
	m, _ := fixture(t)
	m.Update(preparationFailed{commitSelected, context.Canceled})
	if _, ok := m.mode.(browsing); !ok || m.message != "已取消" {
		t.Fatal(m.mode, m.message)
	}
	m.Update(repositoryResult{commitSelected, "", context.Canceled})
	if _, ok := m.mode.(repositoryError); !ok || !strings.Contains(m.repositoryContent(), "可能已完成部分操作") {
		t.Fatal(m.mode, m.repositoryContent())
	}
}
