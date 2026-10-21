package tui

import (
	"fmt"
	"strings"

	"github.com/charmbracelet/x/ansi"
	"gw/internal/app"
	"gw/internal/model"
	"gw/internal/scheduler"
	"gw/internal/state"
)

func (m *screen) orderName() string {
	if m.order == pathOrder {
		return "路径顺序"
	}
	return "问题优先"
}
func (m *screen) header() string {
	title := "仓库"
	switch m.page {
	case detail:
		title = "仓库详情"
	case settings:
		title = "设置"
	case help:
		title = "帮助"
	}
	first := accent.Render(" gw ") + " / " + title
	if m.snapshot.Activity == state.Running {
		first += " · 正在检查"
	}
	if m.page == repositories && m.snapshot.Registration == scheduler.NotLoaded {
		first += " · 定时任务未加载"
	}
	lines := []string{first}
	if m.page == repositories || m.page == detail {
		switch saved := m.snapshot.Report.(type) {
		case app.AvailableReport:
			if m.page == repositories {
				stats := app.Stats(saved.Report)
				lines = append(lines, fmt.Sprintf(" 正常 %d · 需关注 %d · 失败 %d", stats.Counts[model.Clean], stats.Counts[model.Attention], stats.Counts[model.Failed]))
			}
			line := " 上次检查：" + reportTime(saved.Report.FinishedAt)
			if saved.Scope == app.Outdated {
				line += " · 报告范围已变化"
			}
			lines = append(lines, line)
		case app.MissingReport:
			lines = append(lines, " 尚无完整报告")
		}
	}
	for i := range lines {
		lines[i] = ansi.Truncate(lines[i], max(1, m.width-2), "…")
	}
	return strings.Join(lines, "\n")
}
