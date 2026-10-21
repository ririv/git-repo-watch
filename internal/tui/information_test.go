package tui

import (
	"strings"
	"testing"

	tea "charm.land/bubbletea/v2"
	"gw/internal/app"
	"gw/internal/model"
	"gw/internal/scheduler"
	"gw/internal/state"
)

func TestRepositoryScreenSeparatesCountsFromPositionAndSchedule(t *testing.T) {
	m, _ := fixture(t)
	for _, size := range []tea.WindowSizeMsg{{Width: 60, Height: 18}, {Width: 100, Height: 28}, {Width: 150, Height: 32}} {
		m.Update(size)
		m.page = repositories
		m.selected = 2
		header := m.header()
		if strings.Contains(header, "小时") || strings.Contains(header, "个仓库") || strings.Contains(header, "空闲") || strings.Contains(header, "推送成功/失败") {
			t.Fatal(header)
		}
		view := m.View().Content
		if !strings.Contains(view, "全部 3") || !strings.Contains(view, "第 3 项") || strings.Contains(view, "已注册") || strings.Contains(view, "3 / 3") {
			t.Fatal(size, view)
		}
		if strings.Count(view, "上次检查：") != 1 || strings.Contains(m.footer(), "已读取") {
			t.Fatal(view)
		}
	}
}
func TestFilteredCountIsDistinctFromRegisteredCount(t *testing.T) {
	m, _ := fixture(t)
	press(m, "2")
	tabs := m.filtersView()
	if !strings.Contains(tabs, "全部 3") || !strings.Contains(tabs, "问题 1库") {
		t.Fatal(tabs)
	}
	m.prompt(search, "missing")
	tabs = m.filtersView()
	if !strings.Contains(tabs, "匹配 0") || strings.Contains(m.table(70, 10), "第 0 项") {
		t.Fatal(tabs, m.table(70, 10))
	}
}
func TestDetailsKeepEvidenceWithoutRepeatingEmptyMetrics(t *testing.T) {
	m, _ := fixture(t)
	m.page = detail
	m.focusPath("/path/to/clean")
	content := m.detailContent()
	if strings.Count(content, "/path/to/clean") != 1 || strings.Contains(content, "报告时间") || strings.Contains(content, "无推送记录") || strings.Contains(content, "检查通过。") {
		t.Fatal(content)
	}
	if !strings.Contains(content, "experiment") || !strings.Contains(content, "状态：正常") {
		t.Fatal(content)
	}
	m.page = repositories
	if strings.Contains(m.detailContent(), "状态：正常") {
		t.Fatal(m.detailContent())
	}
	saved := m.snapshot.Report.(app.AvailableReport)
	saved.Report.Repositories[0] = model.Normal("/path/to/clean", nil, nil).Record()
	m.snapshot.Report = saved
	content = m.detailContent()
	if strings.Contains(content, "未配置远程上游") || strings.Contains(content, "自动推送") {
		t.Fatal(content)
	}
}
func TestScheduleIsAvailableInSettingsAndWarningsRemainVisible(t *testing.T) {
	m, _ := fixture(t)
	m.page = settings
	if strings.Contains(m.header(), "正常") || strings.Contains(m.header(), "上次检查") {
		t.Fatal(m.header())
	}
	content := m.settingsMenuView()
	if !strings.Contains(content, "每 6 小时") || !strings.Contains(content, "调度已加载") {
		t.Fatal(content)
	}
	m.page = repositories
	m.snapshot.Registration = scheduler.NotLoaded
	m.snapshot.Activity = state.Running
	if !strings.Contains(m.header(), "定时任务未加载") || !strings.Contains(m.header(), "正在检查") {
		t.Fatal(m.header())
	}
}
