package tui

import (
	"fmt"
	"strings"

	"charm.land/lipgloss/v2"
	"gw/internal/model"
)

var (
	good      = lipgloss.NewStyle().Foreground(lipgloss.Color("78"))
	bad       = lipgloss.NewStyle().Foreground(lipgloss.Color("203"))
	published = lipgloss.NewStyle().Foreground(lipgloss.Color("141"))
)

type badge struct {
	text  string
	style lipgloss.Style
}

func (b badge) render() string { return b.style.Render(b.text) }

type repositoryBadges struct{ overall, workspace, remote, experimental, push badge }

// badges projects recorded facts only; failed inspections leave workspace and branch counts unknown.
func badges(r model.Repository) repositoryBadges {
	unknown := badge{"?", muted}
	b := repositoryBadges{badge{"·", muted}, unknown, unknown, unknown, badge{"—", muted}}
	switch r.Status {
	case model.Clean:
		b.overall = badge{"✓", good}
	case model.Attention:
		b.overall = badge{"!", warning}
	case model.Failed:
		b.overall = badge{"×", bad}
	}
	if r.Status == model.Clean || r.Status == model.Attention {
		b.workspace = badge{"干净", good}
		b.remote = badge{"通过", good}
		b.experimental = badge{fmt.Sprint(len(r.Branches)), muted}
		// 同时存在多个问题时，优先展示需要处理的远程问题；详情保留完整问题列表。
		kinds := map[model.IssueKind]struct{}{}
		for _, i := range r.Issues {
			kinds[i.Kind] = struct{}{}
		}
		if _, ok := kinds[model.Dirty]; ok {
			b.workspace = badge{"*改动", warning}
		}
		for _, kind := range []model.IssueKind{model.OutOfSync, model.MissingUpstream, model.PushError} {
			if _, ok := kinds[kind]; ok {
				label := map[model.IssueKind]string{model.OutOfSync: "待同步", model.MissingUpstream: "上游缺失", model.PushError: "推送失败"}[kind]
				b.remote = badge{label, warning}
			}
		}
		if _, ok := kinds[model.NoCommits]; ok {
			b.remote = badge{"无提交", muted}
		}
	}
	if r.Status == model.Failed {
		b.remote = badge{"检查失败", bad}
	}
	success, failed := 0, 0
	for _, p := range r.Pushes {
		switch p.Status {
		case "pushed":
			success++
		case "failed":
			failed++
		}
	}
	if success+failed > 0 {
		style := published
		if failed > 0 {
			style = bad
		}
		b.push = badge{fmt.Sprintf("↑%d !%d", success, failed), style}
	}
	return b
}

func priority(s model.Status) int {
	switch s {
	case model.Failed:
		return 0
	case model.Attention:
		return 1
	case "":
		return 2
	case model.Clean:
		return 3
	}
	return 4
}
func oneLineIssue(r model.Repository) string {
	if r.Error != "" {
		return safe(r.Error)
	}
	parts := []string{}
	for _, i := range r.Issues {
		parts = append(parts, safe(i.Detail))
	}
	if len(parts) > 0 {
		return strings.Join(parts, "；")
	}
	if r.Status == "" {
		return "尚未出现在完整报告中"
	}
	return "已配置远程上游的分支检查通过；无上游分支单独统计"
}
