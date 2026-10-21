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
	if m.width < 90 || m.height < 24 {
		summary := fmt.Sprintf(" %d 个仓库 · 每 %gh · 尚无报告", len(m.config.Repositories), float64(m.config.Interval)/3600)
		if r, ok := m.snapshot.Report.(app.AvailableReport); ok {
			stats := app.Stats(r.Report)
			summary = fmt.Sprintf(" ✓%d 正常 !%d 关注 ×%d 失败 · 每 %gh", stats.Counts[model.Clean], stats.Counts[model.Attention], stats.Counts[model.Failed], float64(m.config.Interval)/3600)
		}
		lines = []string{first, summary}
		if r, ok := m.snapshot.Report.(app.AvailableReport); ok {
			lines = append(lines, " 报告："+reportTime(r.Report.FinishedAt)+" · r 刷新")
		}
		for i := range lines {
			lines[i] = ansi.Truncate(lines[i], max(1, m.width-2), "…")
		}
		return strings.Join(lines, "\n")
	}
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
	case choosing:
		actionLine = "↑↓/jk 选择操作 · Enter 下一步 · Esc 关闭菜单"
	case working:
		actionLine = warning.Render("操作进行中 · Esc/Ctrl+C 取消并等待退出")
	case stopping:
		actionLine = warning.Render("正在取消，请等待操作退出")
	default:
		switch m.page {
		case repositories:
			actionLine = "↑↓选择 Enter详情 /搜索 Space操作 Tab设置 ?帮助 q退出"
		case settings:
			actionLine = "↑↓选择 Enter修改 PgDn滚动 Space操作 Tab仓库 ?帮助"
		default:
			actionLine = "↑↓/PgDn滚动 Esc返回 Space操作 ?帮助 q退出"
		}
	}
	return muted.Render(ansi.Truncate(safe(m.message), max(1, m.width-2), "…")) + "\n" + actionLine
}
func (m *screen) bodyHeight() int {
	return max(1, m.height-lipgloss.Height(m.header())-lipgloss.Height(lipgloss.NewStyle().Width(max(1, m.width-2)).Render(m.footer()))-4)
}
func (m *screen) detailContent() string {
	r, ok := m.selection()
	if !ok {
		return "当前仓库不在筛选结果中；Esc 返回。"
	}
	var b strings.Builder
	fmt.Fprintf(&b, "%s\n%s\n状态：%s\n", safe(filepath.Base(r.Path)), safe(r.Path), statusName(r.Status))
	bgs := badges(r)
	fmt.Fprintf(&b, "工作区 %s · 远程 %s\n无上游 %s · 推送 %s\n\n", bgs.workspace.render(), bgs.remote.render(), bgs.experimental.render(), bgs.push.render())
	if saved, ok := m.snapshot.Report.(app.AvailableReport); ok {
		fmt.Fprintf(&b, "报告时间：%s\n\n", reportTime(saved.Report.FinishedAt))
	}
	if r.Status == "" {
		b.WriteString("上次报告未包含此仓库，等待定时或显式检查。\n")
	}
	if r.Status == model.Clean || r.Status == model.Attention {
		b.WriteString("远程列仅统计已配置远程上游的分支。\n\n")
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

const helpText = `操作说明

仓库页
  ↑↓ / j k：选择仓库；Enter：详情；PgUp/PgDn：翻页
  1：全部；2：需关注和失败；3：无远程上游；4：推送记录
  /：即时按路径搜索；Enter 保留条件，Esc 放弃本次输入
  o：问题优先 / 路径顺序；切换排序和刷新保留选中仓库
  120 列以上且高度足够时，右侧自动预览选中仓库
  Esc：从详情返回时保留列表条件；在列表中清除搜索
  a：添加仓库根目录；d：移除选中的注册项

设置页（Tab 切换）
  ↑↓ / j k：选择设置；Enter：修改；PgUp/PgDn：滚动
  a：添加；x：排除目录；u：取消排除
  s：扫描预览；i：扫描并导入；输入绝对路径
  t：保存频率，例如 6h、30m、1d
  p：保存推送策略，disabled 或 fast_forward
  l：应用系统调度，不立即检查

全局
  Space：操作菜单，↑↓选择、Enter 下一步，Esc 关闭
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

状态图例
  ✓ 正常；! 需关注；× 检查失败；· 未检查
  工作区 *改动：有未提交、未跟踪或子模块改动
  现有报告未区分暂存和未暂存，界面不推测这些细节
  远程：仅展示已配置远程上游的分支检查结果
  无上游：实验分支数，单独统计，不单独构成问题
  推送 ↑n !m：上次报告成功 / 失败的分支数，不是提交数
  ?：检查未完成或没有报告，此项未知；—：没有推送记录
`

func (m *screen) updateContent() {
	m.viewport.SetWidth(max(1, m.width-2))
	m.viewport.SetHeight(m.bodyHeight())
	switch m.page {
	case detail:
		m.viewport.SetContent(m.detailContent())
	case settings:
		m.viewport.SetContent(m.settingsMenuView())
	case help:
		m.viewport.SetContent(helpText)
	}
	m.viewport.GotoTop()
	if m.page == settings {
		m.viewport.EnsureVisible(m.settingSelected+2, 0, 1)
	}
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
	if mode, ok := m.mode.(choosing); ok {
		body = m.menuView(mode.selected)
	}
	content := m.header() + "\n" + muted.Render(strings.Repeat("─", m.width-2)) + "\n" + lipgloss.NewStyle().Height(m.bodyHeight()).Render(body) + "\n" + muted.Render(strings.Repeat("─", m.width-2)) + "\n" + lipgloss.NewStyle().Width(m.width-2).Render(m.footer())
	v := tea.NewView(content)
	v.AltScreen = true
	v.WindowTitle = "gw · Git 仓库"
	return v
}
