package tui

import (
	"fmt"
	"strings"

	tea "charm.land/bubbletea/v2"
	"github.com/charmbracelet/x/ansi"
)

type choosing struct{ selected int }

func (choosing) interaction() {}

type menuItem struct {
	action                  action
	key, title, description string
}

var settingsItems = []menuItem{
	{interval, "t", "检查频率", "保存间隔；随后应用调度才会修改系统任务。"},
	{policy, "p", "自动推送策略", "关闭，或仅发布领先且不落后的已提交内容。"},
	{applySchedule, "l", "应用系统调度", "重新加载系统任务，不会立即检查或推送。"},
	{add, "a", "添加仓库", "指定仓库根目录，加入显式检查列表。"},
	{exclude, "x", "排除目录", "移除该目录下的注册项，保留磁盘文件。"},
	{unexclude, "u", "取消排除", "仅取消排除，不会自动重新添加仓库。"},
	{scan, "s", "扫描预览", "遵守 Git 忽略规则，只生成扫描报告。"},
	{importTree, "i", "扫描并导入", "将发现的仓库加入显式列表；不检查或推送。"},
}

func (m *screen) menuItems() []menuItem {
	items := []menuItem{{check, "c", "立即检查", "检查所有注册仓库，按当前策略自动推送。"}}
	items = append(items, settingsItems...)
	if _, ok := m.selection(); ok {
		items = append(items, menuItem{remove, "d", "移除选中仓库", "仅移除注册项，保留磁盘文件。"})
	}
	return items
}
func (m *screen) activate(a action) tea.Cmd {
	switch a {
	case check, applySchedule:
		m.mode = confirming{a, ""}
		return nil
	case remove:
		if r, ok := m.selection(); ok {
			m.mode = confirming{a, r.Path}
		} else {
			m.mode = browsing{}
			m.message = "没有可移除的仓库"
		}
		return nil
	case interval:
		return m.prompt(a, fmt.Sprintf("%ds", m.config.Interval))
	case policy:
		return m.prompt(a, string(m.config.PushPolicy))
	default:
		return m.prompt(a, "")
	}
}
func (m *screen) menuView(selected int) string {
	items := m.menuItems()
	height := m.bodyHeight()
	capacity := max(1, height-5)
	start := min(max(0, selected-capacity+1), max(0, len(items)-capacity))
	lines := []string{}
	for i := start; i < min(len(items), start+capacity); i++ {
		item := items[i]
		prefix := "  "
		if i == selected {
			prefix = "› "
		}
		line := prefix + cell(item.key, 3) + " " + item.title + "  " + muted.Render(item.description)
		line = ansi.Truncate(line, m.width-6, "…")
		if i == selected {
			line = selectedStyle.Render(line)
		}
		lines = append(lines, line)
	}
	lines = append(lines, "", muted.Render(fmt.Sprintf("%d / %d · %s", selected+1, len(items), items[selected].description)))
	return frame("操作菜单 · 仅包含 gw 的检查与配置操作", strings.Join(lines, "\n"), m.width-2, height)
}
func (m *screen) settingsMenuView() string {
	lines := []string{accent.Render("检查与推送"), ""}
	for i, item := range settingsItems {
		marker := "  "
		if i == m.settingSelected {
			marker = "› "
		}
		value := ""
		switch item.action {
		case interval:
			value = fmt.Sprintf("每 %g 小时", float64(m.config.Interval)/3600)
		case policy:
			value = string(m.config.PushPolicy)
		}
		line := marker + cell(item.key, 3) + " " + cell(item.title, 18) + " " + value
		if i == m.settingSelected {
			line = selectedStyle.Render(line)
		}
		lines = append(lines, line)
	}
	lines = append(lines, "", accent.Render(settingsItems[m.settingSelected].title), settingsItems[m.settingSelected].description, "", accent.Render("排除目录"))
	if len(m.config.Excluded) == 0 {
		lines = append(lines, "无")
	}
	for _, path := range m.config.Excluded {
		lines = append(lines, safe(path))
	}
	lines = append(lines, "", "设置修改不会触发检查或推送；l 应用调度后生效。")
	return strings.Join(lines, "\n")
}
