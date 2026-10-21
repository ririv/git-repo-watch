package tui

import (
	"strings"
	"testing"

	tea "charm.land/bubbletea/v2"
	"charm.land/lipgloss/v2"
	"gw/internal/app"
	"gw/internal/model"
)

func TestFailureSummaryRetainsEvidenceAndAmbiguity(t *testing.T) {
	for _, item := range []struct{ diagnostic, want string }{
		{"git fetch 失败：LibreSSL SSL_connect: SSL_ERROR_SYSCALL in connection to example.com:443", "远程 TLS 连接失败（SSL_ERROR_SYSCALL）"},
		{"main：远程在 fetch 后发生变化或 fetch refspec 未更新上游，请重新检查", "上游引用不一致：远程更新或 fetch 配置未更新该上游"},
		{"git 命令未完成：context deadline exceeded", "git 命令未完成：context deadline exceeded"},
	} {
		if got := failureSummary(item.diagnostic); got != item.want {
			t.Fatal(got, item.want)
		}
	}
}

func TestFailureReasonIsVisibleInListAtEverySupportedSize(t *testing.T) {
	m, _ := fixture(t)
	saved := m.snapshot.Report.(app.AvailableReport)
	saved.Report.Repositories[0] = model.Failure("/path/to/clean", "git fetch 失败：LibreSSL SSL_ERROR_SYSCALL in connection to example.com:443", nil).Record()
	m.snapshot.Report = saved
	m.selected = 0
	for _, size := range []tea.WindowSizeMsg{{Width: 60, Height: 18}, {Width: 80, Height: 24}, {Width: 120, Height: 30}, {Width: 150, Height: 32}} {
		m.Update(size)
		m.page = repositories
		view := m.View().Content
		if !strings.Contains(view, "↳ 远程 TLS 连接失败") || !strings.Contains(view, "SSL_ERROR_SYSCALL") {
			t.Fatal(size, view)
		}
		if lipgloss.Height(view) > size.Height || lipgloss.Width(view) > size.Width {
			t.Fatal(size, view)
		}
	}
	press(m, "enter")
	content := m.detailContent()
	cause := strings.Index(content, "失败原因：远程 TLS 连接失败")
	if cause < 0 || cause > strings.Index(content, "工作区") || !strings.Contains(content, "原始错误：git fetch 失败") {
		t.Fatal(content)
	}
}

func TestFailureRowsKeepSelectedRepositoryAndItsReasonTogether(t *testing.T) {
	m, _ := fixture(t)
	saved := m.snapshot.Report.(app.AvailableReport)
	saved.Report.Repositories = []model.Repository{}
	for _, path := range m.config.Repositories {
		saved.Report.Repositories = append(saved.Report.Repositories, model.Failure(path, "主分支：远程在 fetch 后发生变化或 fetch refspec 未更新上游", nil).Record())
	}
	m.snapshot.Report = saved
	m.Update(tea.WindowSizeMsg{Width: 60, Height: 18})
	for i := range len(m.rows()) {
		m.selected = i
		view := m.listView()
		if !strings.Contains(view, "↳ 上游引用不一致") || !strings.Contains(view, strings.TrimPrefix(m.rows()[i].Path, "/path/to/")) {
			t.Fatal(i, view)
		}
	}
}
