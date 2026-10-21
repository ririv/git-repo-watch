package tui

import (
	"strings"
	"testing"

	tea "charm.land/bubbletea/v2"
	"charm.land/lipgloss/v2"
	"gw/internal/app"
	"gw/internal/model"
)

func TestStatusColumnsUseReportFactsAndKeepUnknowns(t *testing.T) {
	normal := model.Normal("/path/to/repo", []model.BranchWithoutUpstream{{Branch: "experiment", Reason: model.NotConfigured}}, nil).Record()
	b := badges(normal)
	if b.overall.text != "✓" || b.workspace.text != "干净" || b.remote.text != "通过" || b.experimental.text != "1" {
		t.Fatal(b)
	}
	dirty := model.NeedsAttention(normal.Path, model.Issue{Kind: model.Dirty, Detail: "未提交"}, []model.Issue{{Kind: model.OutOfSync, Detail: "展示文本，不解析数量"}}, nil, nil).Record()
	b = badges(dirty)
	if b.workspace.text != "*改动" || b.remote.text != "待同步" {
		t.Fatal(b)
	}
	failed := model.Failure(normal.Path, "远程查询失败", []model.PushRecord{model.Published(model.PushTarget{}).Record()}).Record()
	b = badges(failed)
	if b.workspace.text != "?" || b.experimental.text != "?" || b.remote.text != "检查失败" || b.push.text != "↑1 !0" {
		t.Fatal(b)
	}
	b = badges(model.Repository{Path: normal.Path})
	if b.overall.text != "·" || b.workspace.text != "?" || b.experimental.text != "?" {
		t.Fatal(b)
	}
}
func TestOperationMenuRequiresExplicitActionAndConfirmation(t *testing.T) {
	m, s := fixture(t)
	press(m, " ")
	if _, ok := m.mode.(choosing); !ok {
		t.Fatal(m.mode)
	}
	press(m, "enter")
	if mode, ok := m.mode.(confirming); !ok || mode.action != check {
		t.Fatal(m.mode)
	}
	press(m, "n")
	press(m, " ")
	press(m, "down")
	press(m, "enter")
	mode, ok := m.mode.(editing)
	if !ok || mode.action != interval || mode.input.Value() != "21600s" {
		t.Fatal(m.mode)
	}
	press(m, "esc")
	c, e := s.Config()
	if e != nil || c.Interval != 21600 {
		t.Fatal(c, e)
	}
	press(m, " ")
	press(m, "esc")
	if _, ok := m.mode.(browsing); !ok {
		t.Fatal(m.mode)
	}
}
func TestSettingsCanBeSelectedWithoutRememberingShortcuts(t *testing.T) {
	m, _ := fixture(t)
	press(m, "tab")
	press(m, "down")
	press(m, "enter")
	mode, ok := m.mode.(editing)
	if !ok || mode.action != policy {
		t.Fatal(m.mode)
	}
	press(m, "esc")
	for range 6 {
		press(m, "down")
	}
	if m.settingSelected != 7 || !strings.Contains(m.viewport.View(), "扫描并导入") {
		t.Fatal(m.settingSelected, m.viewport.View())
	}
	press(m, "?")
	press(m, "esc")
	if m.page != settings || m.settingSelected != 7 {
		t.Fatal(m.page, m.settingSelected)
	}
}
func TestRefreshAndSortKeepSelectedRepositoryByPath(t *testing.T) {
	m, s := fixture(t)
	m.focusPath("/path/to/clean")
	press(m, "o")
	if m.order != pathOrder || m.selectedPath() != "/path/to/clean" {
		t.Fatal(m.order, m.selectedPath())
	}
	press(m, "o")
	if m.order != priorityOrder || m.selectedPath() != "/path/to/clean" {
		t.Fatal(m.order, m.selectedPath())
	}
	r, e := s.Report()
	if e != nil {
		t.Fatal(e)
	}
	r.Repositories[0] = model.Failure("/path/to/clean", "网络失败", nil).Record()
	if e = s.Store.WriteReport(r); e != nil {
		t.Fatal(e)
	}
	settle(m, press(m, "r"))
	if m.selectedPath() != "/path/to/clean" || m.selected != 0 {
		t.Fatal(m.selectedPath(), m.selected)
	}
}
func TestLiveSearchCancelAndBackNavigation(t *testing.T) {
	m, _ := fixture(t)
	m.prompt(search, "")
	for _, r := range "dirty" {
		m.Update(tea.KeyPressMsg{Code: r, Text: string(r)})
	}
	if len(m.rows()) != 1 || m.query != "" {
		t.Fatal(m.rows(), m.query)
	}
	press(m, "esc")
	if len(m.rows()) != 3 {
		t.Fatal(m.rows())
	}
	m.prompt(search, "dirty")
	press(m, "enter")
	press(m, "enter")
	press(m, "?")
	press(m, "esc")
	if m.page != detail || m.query != "dirty" {
		t.Fatal(m.page, m.query)
	}
	press(m, "esc")
	if m.page != repositories || m.query != "dirty" {
		t.Fatal(m.page, m.query)
	}
	press(m, "esc")
	if m.query != "" || m.selectedPath() != "/path/to/dirty" {
		t.Fatal(m.query, m.selectedPath())
	}
}
func TestResponsiveTablePreviewAndMenusRemainUsable(t *testing.T) {
	m, _ := fixture(t)
	for _, size := range []tea.WindowSizeMsg{{Width: 60, Height: 18}, {Width: 80, Height: 24}, {Width: 120, Height: 30}, {Width: 150, Height: 32}} {
		m.Update(size)
		m.page = repositories
		m.mode = browsing{}
		view := m.View().Content
		if !strings.Contains(view, "dirty") || !strings.Contains(view, "工作区") || !strings.Contains(view, "无上游") {
			t.Fatal(size, view)
		}
		if (size.Width >= 120) != strings.Contains(view, "选中仓库") {
			t.Fatal(size, view)
		}
		for _, mode := range []interaction{browsing{}, choosing{selected: 8}, confirming{check, ""}, confirming{remove, "/path/to/repository"}} {
			m.mode = mode
			view = m.View().Content
			if lipgloss.Height(view) > size.Height || lipgloss.Width(view) > size.Width {
				t.Fatalf("size %+v mode %T produced %dx%d", size, mode, lipgloss.Width(view), lipgloss.Height(view))
			}
		}
	}
}
func TestPrioritySortingDoesNotChangeRegisteredConfiguration(t *testing.T) {
	m, s := fixture(t)
	before, _ := s.Config()
	press(m, "o")
	press(m, "o")
	after, _ := s.Config()
	if strings.Join(before.Repositories, "\n") != strings.Join(after.Repositories, "\n") {
		t.Fatal(before, after)
	}
	m.filter = app.WithoutUpstream
	b := badges(m.rows()[0])
	if b.overall.text != "✓" || b.experimental.text != "1" {
		t.Fatal(b)
	}
}
