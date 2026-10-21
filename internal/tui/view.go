package tui

import (
	"fmt"
	"path/filepath"
	"strings"
	"time"
	"unicode"

	tea "charm.land/bubbletea/v2"
	"charm.land/lipgloss/v2"
	"github.com/charmbracelet/x/ansi"
	"gw/internal/app"
	"gw/internal/model"
	"gw/internal/scheduler"
	"gw/internal/state"
)

var (
	accent        = lipgloss.NewStyle().Foreground(lipgloss.Color("75")).Bold(true)
	muted         = lipgloss.NewStyle().Foreground(lipgloss.Color("245"))
	selectedStyle = lipgloss.NewStyle().Background(lipgloss.Color("24")).Foreground(lipgloss.Color("255"))
	warning       = lipgloss.NewStyle().Foreground(lipgloss.Color("214"))
)

// safe treats repository names and command output as text, never terminal commands.
func safe(s string) string {
	return strings.Map(func(r rune) rune {
		if unicode.IsControl(r) {
			return ' '
		}
		return r
	}, ansi.Strip(s))
}
func reportTime(value string) string {
	t, e := time.Parse(time.RFC3339Nano, value)
	if e != nil {
		return "报告时间格式无效：" + safe(value)
	}
	return t.Local().Format("2006-01-02 15:04:05 MST")
}
func statusName(s model.Status) string {
	switch s {
	case model.Clean:
		return "正常"
	case model.Attention:
		return "需关注"
	case model.Failed:
		return "检查失败"
	case "":
		return "未检查"
	}
	return "未知状态"
}
func filterName(f app.Filter) string {
	switch f {
	case app.All:
		return "全部"
	case app.Problems:
		return "问题"
	case app.WithoutUpstream:
		return "无上游"
	case app.Pushes:
		return "推送"
	}
	return "未知筛选"
}
func (m *screen) header() string {
	tab := "仓库"
	switch m.page {
	case detail:
		tab = "仓库详情"
	case settings:
		tab = "设置"
	case help:
		tab = "帮助"
	}
	first := accent.Render(" gw ") + " / " + tab + "    " + muted.Render("Tab 仓库/设置 · ? 帮助 · q 退出")
	registration := "未加载"
	if m.snapshot.Registration == scheduler.Loaded {
		registration = "已加载"
	}
	activity := "空闲"
	if m.snapshot.Activity == state.Running {
		activity = "检查运行中"
	}
	second := fmt.Sprintf(" 每 %g 小时 · %d 个仓库 · 定时任务%s · %s", float64(m.config.Interval)/3600, len(m.config.Repositories), registration, activity)
	third := " 尚无完整报告；进入界面不会检查"
	if r, ok := m.snapshot.Report.(app.AvailableReport); ok {
		s := app.Stats(r.Report)
		third = fmt.Sprintf(" 正常 %d · 需关注 %d · 失败 %d · 无上游 %d 分支 · 推送成功/失败 %d/%d", s.Counts[model.Clean], s.Counts[model.Attention], s.Counts[model.Failed], s.Branches.Branches, s.Pushes["pushed"], s.Pushes["failed"])
		if r.Scope == app.Outdated {
			third += " · 报告范围已变化"
		}
	}
	lines := []string{first, second, third}
	if r, ok := m.snapshot.Report.(app.AvailableReport); ok {
		line := " 报告：" + reportTime(r.Report.FinishedAt)
		if app.Stats(r.Report).Branches.Unavailable > 0 {
			line += " · 失败仓库分支统计不完整"
		}
		lines = append(lines, line)
	}
	for i := range lines {
		lines[i] = ansi.Truncate(lines[i], max(1, m.width-2), "…")
	}
	return strings.Join(lines, "\n")
}
func (m *screen) footer() string {
	var actionLine string
	switch mode := m.mode.(type) {
	case editing:
		actionLine = actionTitle(mode.action) + "\n" + mode.input.View() + "\nEnter 下一步 · Esc 取消"
	case confirming:
		text := actionTitle(mode.action)
		if mode.value != "" {
			text += "：" + safe(mode.value)
		}
		if mode.action == check {
			text += "；当前推送策略：" + string(m.config.PushPolicy)
		}
		actionLine = warning.Render(text) + "\n[y] 确认 · [n/Esc] 取消"
	case working:
		actionLine = warning.Render("操作进行中 · Esc/Ctrl+C 取消并等待退出")
	case stopping:
		actionLine = warning.Render("正在取消，请等待操作退出")
	default:
		switch m.page {
		case repositories:
			actionLine = "↑↓/jk 选择 · Enter 详情 · / 搜索 · 1–4 筛选 · a 添加 · d 移除\nr 刷新已有报告 · c 手动检查 · Esc 清除搜索"
		case settings:
			actionLine = "a 添加 · x 排除 · u 取消排除 · s 扫描 · i 导入\nt 频率 · p 推送策略 · l 应用调度 · r 刷新 · c 手动检查"
		default:
			actionLine = "↑↓/jk / PgUp/PgDn 滚动 · Esc 返回仓库 · r 刷新 · c 手动检查"
		}
	}
	return muted.Render(ansi.Truncate(safe(m.message), max(1, m.width-2), "…")) + "\n" + actionLine
}
func (m *screen) bodyHeight() int {
	return max(1, m.height-lipgloss.Height(m.header())-lipgloss.Height(lipgloss.NewStyle().Width(max(1, m.width-2)).Render(m.footer()))-4)
}
func (m *screen) listView() string {
	var b strings.Builder
	title := " 1 全部   2 问题   3 无上游   4 推送   当前：" + filterName(m.filter)
	if m.query != "" {
		title += " · 搜索：" + safe(m.query)
	}
	b.WriteString(ansi.Truncate(title, m.width-2, "…") + "\n\n")
	rows := m.rows()
	h := max(1, m.bodyHeight()-3)
	if len(rows) == 0 {
		return b.String() + " 暂无匹配仓库；可按 a 添加，或 Tab 进入设置。"
	}
	start := min(max(0, m.selected-h+1), max(0, len(rows)-h))
	for i := start; i < min(len(rows), start+h); i++ {
		r := rows[i]
		marker := "  "
		if i == m.selected {
			marker = "› "
		}
		prefix := marker + lipgloss.NewStyle().Width(10).Render(statusName(r.Status)) + " " + safe(filepath.Base(r.Path))
		line := prefix + "  " + muted.Render(safe(r.Path))
		line = ansi.Truncate(line, max(1, m.width-2), "…")
		if i == m.selected {
			line = selectedStyle.Width(max(1, m.width-2)).Render(line)
		}
		b.WriteString(line + "\n")
	}
	b.WriteString(muted.Render(fmt.Sprintf(" %d / %d", m.selected+1, len(rows))))
	return b.String()
}
func (m *screen) detailContent() string {
	r, ok := m.selection()
	if !ok {
		return "当前仓库不在筛选结果中；Esc 返回。"
	}
	var b strings.Builder
	fmt.Fprintf(&b, "%s\n%s\n状态：%s\n\n", safe(filepath.Base(r.Path)), safe(r.Path), statusName(r.Status))
	if saved, ok := m.snapshot.Report.(app.AvailableReport); ok {
		fmt.Fprintf(&b, "报告时间：%s\n\n", reportTime(saved.Report.FinishedAt))
	}
	if r.Status == "" {
		b.WriteString("上次报告未包含此仓库，等待定时或显式检查。\n")
	}
	for _, i := range r.Issues {
		fmt.Fprintf(&b, "• %s\n", safe(i.Detail))
	}
	if r.Error != "" {
		fmt.Fprintf(&b, "错误：%s\n", safe(r.Error))
	}
	b.WriteString("\n未配置远程上游（单独统计）\n")
	if len(r.Branches) == 0 {
		if r.Status == model.Failed || r.Status == "" {
			b.WriteString("无法统计\n")
		} else {
			b.WriteString("无\n")
		}
	}
	for _, branch := range r.Branches {
		reason := "未配置上游"
		if branch.Reason == model.LocalUpstream {
			reason = "上游是本地分支"
		}
		fmt.Fprintf(&b, "• %s：%s\n", safe(branch.Branch), reason)
	}
	b.WriteString("\n自动推送\n")
	if len(r.Pushes) == 0 {
		b.WriteString("无推送记录\n")
	}
	for _, p := range r.Pushes {
		outcome := "推送失败"
		if p.Status == "pushed" {
			outcome = "已推送"
		}
		fmt.Fprintf(&b, "• %s → %s/%s：%s（%d 个提交）\n", safe(p.Target.Branch), safe(p.Target.Remote), safe(strings.TrimPrefix(p.Target.RemoteRef, "refs/heads/")), outcome, p.Target.Ahead)
		if p.Error != "" {
			fmt.Fprintf(&b, "  %s：%s\n", safe(string(p.Phase)), safe(p.Error))
		}
	}
	if saved, ok := m.snapshot.Report.(app.AvailableReport); ok {
		for _, n := range saved.Report.Notifications {
			if n.Path == r.Path {
				fmt.Fprintf(&b, "通知 %s：%s %s\n", safe(n.Branch), safe(n.Status), safe(n.Error))
			}
		}
	}
	return b.String()
}
func (m *screen) settingsContent() string {
	var b strings.Builder
	fmt.Fprintf(&b, "检查频率\n  每 %g 小时（%d 秒）\n  修改后按 l 应用系统调度，不会立即检查。\n\n自动推送\n  %s\n  disabled：关闭；fast_forward：仅领先且不落后时普通推送。\n\n排除目录\n", float64(m.config.Interval)/3600, m.config.Interval, safe(string(m.config.PushPolicy)))
	if saved, ok := m.snapshot.Schedule.(scheduler.Saved); ok && saved.Interval != m.config.Interval {
		b.WriteString("  注意：配置频率与已安装的系统任务不同。\n")
	}
	if len(m.config.Excluded) == 0 {
		b.WriteString("  无\n")
	}
	for _, path := range m.config.Excluded {
		b.WriteString("  " + safe(path) + "\n")
	}
	b.WriteString("\n扫描遵守 Git 忽略规则；导入只更新显式仓库列表。\n排除目录会移除其下注册项，不删除磁盘文件。\n设置修改不会触发检查或推送。\n")
	return b.String()
}

const helpText = `操作说明

仓库页
  ↑↓ / j k：选择仓库；Enter：详情；PgUp/PgDn：翻页
  1：全部；2：需关注和失败；3：无远程上游；4：推送记录
  /：按路径搜索；Esc：清除搜索并返回仓库页
  a：添加仓库根目录；d：移除选中的注册项

设置页（Tab 切换）
  a：添加；x：排除目录；u：取消排除
  s：扫描预览；i：扫描并导入；输入绝对路径
  t：保存频率，例如 6h、30m、1d
  p：保存推送策略，disabled 或 fast_forward
  l：应用系统调度，不立即检查

全局
  r：重新读取已有报告，不访问远程
  c：显式检查；按当前策略自动推送；确认后才执行
  耗时操作期间 Esc/Ctrl+C：取消并等待 Git 子进程退出
  输入时 Esc：取消；确认时 y：执行，n/Esc：取消
  q/Ctrl+C：退出并恢复终端；?：帮助

进入界面不会检查、扫描或推送。
本机报告、日志、路径和配置不进入 Git。
无上游分支单独统计，不作为问题，也不会自动发布。
检查失败的仓库，其分支统计不完整。
定时任务可在界面运行期间执行；按 r 查看最新结果。
`

func (m *screen) updateContent() {
	m.viewport.SetWidth(max(1, m.width-2))
	m.viewport.SetHeight(m.bodyHeight())
	switch m.page {
	case detail:
		m.viewport.SetContent(m.detailContent())
	case settings:
		m.viewport.SetContent(m.settingsContent())
	case help:
		m.viewport.SetContent(helpText)
	}
	m.viewport.GotoTop()
}
func (m *screen) View() tea.View {
	if m.width < 60 || m.height < 18 {
		v := tea.NewView("gw：请将终端调整到至少 60 列 × 18 行。\nq 退出；进行中的操作可按 Esc 取消。")
		v.AltScreen = true
		return v
	}
	m.viewport.SetHeight(m.bodyHeight())
	body := m.viewport.View()
	if m.page == repositories {
		body = m.listView()
	}
	content := m.header() + "\n" + muted.Render(strings.Repeat("─", m.width-2)) + "\n" + lipgloss.NewStyle().Height(m.bodyHeight()).Render(body) + "\n" + muted.Render(strings.Repeat("─", m.width-2)) + "\n" + lipgloss.NewStyle().Width(m.width-2).Render(m.footer())
	v := tea.NewView(content)
	v.AltScreen = true
	v.WindowTitle = "gw · Git 仓库"
	return v
}
