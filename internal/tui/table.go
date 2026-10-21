package tui

import (
	"fmt"
	"path/filepath"
	"strings"

	"charm.land/bubbles/v2/viewport"
	"charm.land/lipgloss/v2"
	"github.com/charmbracelet/x/ansi"
	"gw/internal/app"
)

func cell(text string, width int) string {
	text = ansi.Truncate(text, max(1, width), "…")
	return text + strings.Repeat(" ", max(0, width-lipgloss.Width(text)))
}
func frame(title, content string, width, height int) string {
	inner := max(1, width-2)
	title = ansi.Truncate(" "+title+" ", inner, "…")
	lines := []string{muted.Render("╭" + title + strings.Repeat("─", max(0, inner-lipgloss.Width(title))) + "╮")}
	body := strings.Split(content, "\n")
	for i := 0; i < max(0, height-2); i++ {
		text := ""
		if i < len(body) {
			text = body[i]
		}
		lines = append(lines, muted.Render("│")+cell(text, inner)+muted.Render("│"))
	}
	lines = append(lines, muted.Render("╰"+strings.Repeat("─", inner)+"╯"))
	return strings.Join(lines, "\n")
}
func (m *screen) filtersView() string {
	parts := []string{}
	for i, f := range []app.Filter{app.All, app.Problems, app.WithoutUpstream, app.Pushes} {
		label := filterName(f)
		if f == app.All {
			label += fmt.Sprintf(" %d", len(m.config.Repositories))
		} else if f == m.filter {
			label += fmt.Sprintf(" %d库", len(m.rows()))
		}
		title := fmt.Sprintf(" %d %s ", i+1, label)
		if m.filter == f {
			title = selectedStyle.Bold(true).Render(title)
		} else {
			title = muted.Render(title)
		}
		parts = append(parts, title)
	}
	text := strings.Join(parts, " ")
	if query := m.searchQuery(); query != "" {
		text += fmt.Sprintf("  / %s · 匹配 %d", safe(query), len(m.rows()))
	}

	return ansi.Truncate(text, m.width-2, "…")
}
func (m *screen) table(width, height int) string {
	// 固定状态列，剩余宽度分配给仓库名。全路径放在预览和选中项摘要中。
	nameWidth := max(8, width-35)
	header := cell("", 2) + " " + cell("仓库", nameWidth) + " " + cell("工作区", 6) + " " + cell("远程", 8) + " " + cell("无上游", 6) + " " + cell("推送", 7)
	lines := []string{muted.Render(header)}
	if height >= 7 {
		lines = append(lines, muted.Render(strings.Repeat("─", width)))
	}
	footerHeight := 1
	if height >= 8 && !m.splitView() {
		footerHeight = 2
	}
	rows := m.rows()
	capacity := max(1, height-len(lines)-footerHeight)
	if len(rows) == 0 {
		text := "没有匹配仓库；/ 调整搜索，Esc 清除搜索。"
		if len(m.config.Repositories) == 0 {
			text = "尚未添加仓库；a 添加，或 Tab 进入设置导入。"
		}
		lines = append(lines, ansi.Truncate(text, width, "…"))
	} else {
		start := min(max(0, m.selected-capacity+1), max(0, len(rows)-capacity))
		for i := start; i < min(len(rows), start+capacity); i++ {
			r := rows[i]
			b := badges(r)
			indicator := " "
			if i == m.selected {
				indicator = "›"
			}
			cols := []string{cell(indicator+b.overall.render(), 2), cell(safe(filepath.Base(r.Path)), nameWidth), cell(b.workspace.render(), 6), cell(b.remote.render(), 8), cell(b.experimental.render(), 6), cell(b.push.render(), 7)}
			line := strings.Join(cols, " ")
			if i == m.selected {
				line = selectedStyle.Render(line)
			}
			lines = append(lines, line)
		}
	}
	for len(lines) < height-footerHeight {
		lines = append(lines, "")
	}
	position := 0
	if len(rows) > 0 {
		position = m.selected + 1
	}
	positionText := ""
	if position > 0 {
		positionText = fmt.Sprintf("第 %d 项", position)
	}
	lines = append(lines, muted.Render(positionText))
	if footerHeight == 2 && !m.splitView() {
		if r, ok := m.selection(); ok {
			lines = append(lines, muted.Render(ansi.Truncate(safe(r.Path), width, "…")))
		}
	}
	return strings.Join(lines, "\n")
}
func (m *screen) splitView() bool { return m.width >= 120 && m.bodyHeight() >= 13 }
func (m *screen) listView() string {
	height := max(3, m.bodyHeight()-2)
	width := m.width - 2
	legend := muted.Render(ansi.Truncate("* 工作区改动 · ? 未知 · ↑/! 推送成功/失败分支数", width, "…"))
	if !m.splitView() {
		return m.filtersView() + "\n" + legend + "\n" + frame("列表 · "+m.orderName(), m.table(width-2, height-2), width, height)
	}
	leftWidth := (width - 1) * 55 / 100
	rightWidth := width - leftWidth - 1
	left := frame("列表 · "+m.orderName(), m.table(leftWidth-2, height-2), leftWidth, height)
	preview := viewport.New(viewport.WithWidth(max(1, rightWidth-4)), viewport.WithHeight(max(1, height-4)))
	preview.SoftWrap = true
	preview.SetContent(m.detailContent())
	right := frame("选中仓库", preview.View(), rightWidth, height)
	return m.filtersView() + "\n" + legend + "\n" + lipgloss.JoinHorizontal(lipgloss.Top, left, " ", right)
}
