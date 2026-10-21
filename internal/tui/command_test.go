package tui

import (
	"path/filepath"
	"strings"
	"testing"

	tea "charm.land/bubbletea/v2"
	"charm.land/lipgloss/v2"
	"gw/internal/testkit"
)

func enterCommand(t *testing.T, m *screen, value string) tea.Cmd {
	t.Helper()
	press(m, ":")
	mode, ok := m.mode.(commandEditing)
	if !ok {
		t.Fatal(m.mode)
	}
	mode.input.SetValue(value)
	m.mode = mode
	return press(m, "enter")
}
func TestCommandsCompletionUnknownAndScope(t *testing.T) {
	m, _ := fixture(t)
	press(m, ":")
	mode := m.mode.(commandEditing)
	mode.input.SetValue("sta")
	m.mode = mode
	press(m, "tab")
	if got := m.mode.(commandEditing).input.Value(); got != "stage" {
		t.Fatal(got)
	}
	mode = m.mode.(commandEditing)
	mode.input.SetValue("check")
	m.mode = mode
	if cmd := press(m, "enter"); cmd != nil {
		t.Fatal("未知命令不能执行")
	}
	if _, ok := m.mode.(commandEditing); !ok {
		t.Fatal(m.mode)
	}
	press(m, "esc")
	enterCommand(t, m, "check all")
	if mode, ok := m.mode.(confirming); !ok || mode.action != check {
		t.Fatal(m.mode)
	}
}
func TestFilePanelStageDiffUnstageAndCommit(t *testing.T) {
	m, f := repositoryScreen(t)
	testkit.Write(t, filepath.Join(f.Repo, "one.txt"), "one\n")
	testkit.Write(t, filepath.Join(f.Repo, "two.txt"), "two\n")
	cmd := enterCommand(t, m, "stage")
	m.Update(cmd())
	session := m.mode.(filesBrowsing).session
	if len(session.rows()) != 2 {
		t.Fatal(session.rows())
	}
	press(m, "space")
	cmd = press(m, "enter")
	m.Update(cmd())
	if mode, ok := m.mode.(fileDiff); !ok || !strings.Contains(mode.content, "+one") {
		t.Fatal(m.mode)
	}
	press(m, "esc")
	press(m, ":")
	press(m, "esc")
	if len(m.mode.(filesBrowsing).session.selected) != 1 {
		t.Fatal("关闭命令栏应保留勾选")
	}
	cmd = enterCommand(t, m, "stage")
	m.Update(cmd())
	if got := testkit.Git(t, f.Repo, "diff", "--cached", "--name-only"); got != "one.txt" {
		t.Fatal(got)
	}
	press(m, "tab")
	cmd = enterCommand(t, m, "unstage")
	m.Update(cmd())
	if got := testkit.Git(t, f.Repo, "diff", "--cached", "--name-only"); got != "" {
		t.Fatal(got)
	}
	press(m, "tab")
	cmd = enterCommand(t, m, "stage")
	m.Update(cmd())
	cmd = enterCommand(t, m, "commit")
	m.Update(cmd())
	mode := m.mode.(commitEditing)
	mode.input.SetValue("from command bar")
	m.mode = mode
	press(m, "enter")
	settle(m, press(m, "y"))
	m.workers.Wait()
	if got := testkit.Git(t, f.Repo, "log", "-1", "--format=%s"); got != "from command bar" {
		t.Fatal(got)
	}
	if got := testkit.Git(t, f.Repo, "status", "--porcelain"); !strings.Contains(got, "two.txt") {
		t.Fatal(got)
	}
	cmd = enterCommand(t, m, "push")
	m.Update(cmd())
	if _, ok := m.mode.(pushConfirming); !ok {
		t.Fatal(m.mode)
	}
	settle(m, press(m, "y"))
	m.workers.Wait()
	if got, want := testkit.Git(t, f.Remote, "rev-parse", "main"), testkit.Git(t, f.Repo, "rev-parse", "HEAD"); got != want {
		t.Fatal(got, want)
	}
}
func TestFileAndCommandViewsFitSmallTerminals(t *testing.T) {
	m, f := repositoryScreen(t)
	testkit.Write(t, filepath.Join(f.Repo, "long.txt"), strings.Repeat("line\n", 100))
	cmd := enterCommand(t, m, "stage")
	m.Update(cmd())
	files := m.mode
	cmd = press(m, "enter")
	m.Update(cmd())
	diff := m.mode
	m.mode = files
	press(m, ":")
	command := m.mode
	for _, mode := range []interaction{files, diff, command} {
		for _, size := range [][2]int{{60, 18}, {80, 24}, {150, 32}} {
			m.mode = mode
			m.Update(tea.WindowSizeMsg{Width: size[0], Height: size[1]})
			view := m.View().Content
			if lipgloss.Width(view) > size[0] || lipgloss.Height(view) > size[1] {
				t.Fatalf("%T %v: %dx%d\n%s", mode, size, lipgloss.Width(view), lipgloss.Height(view), view)
			}
		}
	}
}

func TestCommandKeepsOriginalRepositoryScope(t *testing.T) {
	m, _ := fixture(t)
	path := m.selectedPath()
	press(m, ":")
	mode := m.mode.(commandEditing)
	mode.input.SetValue("remove")
	m.mode = mode
	m.selected = 2
	press(m, "enter")
	if mode, ok := m.mode.(confirming); !ok || mode.value != path {
		t.Fatal(m.mode, path)
	}
}
