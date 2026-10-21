package tui

import (
	"context"
	"errors"
	"fmt"
	"strings"

	"charm.land/bubbles/v2/textinput"
	tea "charm.land/bubbletea/v2"
	"gw/internal/repoaction"
)

type commitPrepared struct{ plan repoaction.CommitPlan }
type pushPrepared struct{ plan repoaction.PushPlan }
type preparationFailed struct {
	operation action
	err       error
}
type repositoryResult struct {
	operation action
	message   string
	err       error
}
type commitEditing struct {
	plan  repoaction.CommitPlan
	input textinput.Model
}
type commitConfirming struct {
	plan    repoaction.CommitPlan
	message string
}
type pushConfirming struct{ plan repoaction.PushPlan }
type repositoryError struct {
	operation action
	detail    string
}

func (commitEditing) interaction()    {}
func (commitConfirming) interaction() {}
func (pushConfirming) interaction()   {}
func (repositoryError) interaction()  {}

func (m *screen) prepareRepository(a action) tea.Cmd {
	return m.prepareRepositoryAt(a, m.selectedPath())
}
func (m *screen) prepareRepositoryAt(a action, path string) tea.Cmd {
	if path == "" {
		m.message = "请先选择一个仓库"
		m.mode = browsing{}
		return nil
	}
	cmd := m.startTask(a, func(ctx context.Context) tea.Msg {
		if a == commitSelected {
			p, e := m.backend.PrepareCommit(ctx, path)
			if e != nil {
				return preparationFailed{a, e}
			}
			return commitPrepared{p}
		}
		p, e := m.backend.PreparePush(ctx, path)
		if e != nil {
			return preparationFailed{a, e}
		}
		return pushPrepared{p}
	})
	if a == commitSelected {
		m.message = "正在读取选中仓库的暂存内容…"
	} else {
		m.message = "正在核对当前分支和远程上游…"
	}
	return cmd
}
func (m *screen) closeRepositoryReview() {
	m.mode = browsing{}
	m.message = "已取消"
	m.updateContent()
}
func (m *screen) updateRepository(msg tea.Msg) (tea.Cmd, bool) {
	switch msg := msg.(type) {
	case commitPrepared:
		input := textinput.New()
		input.Prompt = "› "
		input.SetWidth(max(1, m.width-6))
		input.SetVirtualCursor(true)
		cmd := input.Focus()
		m.mode = commitEditing{msg.plan, input}
		m.message = ""
		m.updateContent()
		return cmd, true
	case pushPrepared:
		m.mode = pushConfirming{msg.plan}
		m.message = ""
		m.updateContent()
		return nil, true
	case preparationFailed:
		if errors.Is(msg.err, context.Canceled) {
			m.closeRepositoryReview()
			return nil, true
		}
		m.mode = repositoryError{msg.operation, msg.err.Error()}
		m.message = ""
		m.updateContent()
		return nil, true
	case repositoryResult:
		if msg.err != nil {
			text := msg.err.Error()
			if errors.Is(msg.err, context.Canceled) {
				text = "操作被中断，Git 可能已完成部分操作，请核对提交记录或远程状态。\n" + text
			}
			m.mode = repositoryError{msg.operation, text}
			m.message = ""
			m.updateContent()
			return nil, true
		}
		_, cmd := m.Update(completed{message: msg.message})
		return cmd, true
	case tea.KeyPressMsg:
		key := msg.String()
		switch mode := m.mode.(type) {
		case commitEditing:
			switch key {
			case "esc":
				m.closeRepositoryReview()
				return nil, true
			case "ctrl+c":
				return tea.Quit, true
			case "pgup", "pgdown":
				var cmd tea.Cmd
				m.viewport, cmd = m.viewport.Update(msg)
				return cmd, true
			case "enter":
				if strings.TrimSpace(mode.input.Value()) == "" {
					m.message = "请输入提交说明"
					return nil, true
				}
				m.mode = commitConfirming{mode.plan, mode.input.Value()}
				m.message = ""
				m.updateContent()
				return nil, true
			}
			var cmd tea.Cmd
			mode.input, cmd = mode.input.Update(msg)
			m.mode = mode
			return cmd, true
		case commitConfirming:
			switch key {
			case "y":
				return m.startTask(commitSelected, func(ctx context.Context) tea.Msg {
					r, e := m.backend.CommitRepository(ctx, mode.plan, mode.message)
					if e != nil {
						return repositoryResult{commitSelected, "", e}
					}
					text := "已提交 " + r.OID[:min(12, len(r.OID))] + "；列表仍为上次检查结果"
					if r.Review == repoaction.ChangedCommit {
						text = "已提交 " + r.OID[:min(12, len(r.OID))] + "；内容被钩子或并发操作更改，请查看实际提交"
					}
					return repositoryResult{commitSelected, text, nil}
				}), true
			case "n", "esc":
				m.closeRepositoryReview()
				return nil, true
			case "ctrl+c":
				return tea.Quit, true
			}
			var cmd tea.Cmd
			m.viewport, cmd = m.viewport.Update(msg)
			return cmd, true
		case pushConfirming:
			switch key {
			case "y":
				return m.startTask(pushSelected, func(ctx context.Context) tea.Msg {
					r, e := m.backend.PushRepository(ctx, mode.plan)
					if e != nil {
						return repositoryResult{pushSelected, "", e}
					}
					record := r.Record()
					if record.Status == "failed" {
						return repositoryResult{pushSelected, "", fmt.Errorf("%s 阶段：%s", record.Phase, record.Error)}
					}
					target := mode.plan.Target()
					return repositoryResult{pushSelected, fmt.Sprintf("已推送 %s → %s/%s；列表仍为上次检查结果", target.Branch, target.Remote, strings.TrimPrefix(target.RemoteRef, "refs/heads/")), nil}
				}), true
			case "n", "esc":
				m.closeRepositoryReview()
				return nil, true
			case "ctrl+c":
				return tea.Quit, true
			}
			var cmd tea.Cmd
			m.viewport, cmd = m.viewport.Update(msg)
			return cmd, true
		case repositoryError:
			if key == "esc" || key == "enter" {
				m.mode = browsing{}
				m.updateContent()
				return nil, true
			}
			if key == "q" || key == "ctrl+c" {
				return tea.Quit, true
			}
			var cmd tea.Cmd
			m.viewport, cmd = m.viewport.Update(msg)
			return cmd, true
		}
	}
	return nil, false
}

func commitReview(p repoaction.CommitPlan) string {
	var b strings.Builder
	fmt.Fprintf(&b, "仓库：%s\n分支：%s\n范围：仅已暂存内容\n\n暂存文件\n", safe(p.Path()), safe(p.Branch()))
	for _, file := range p.Files() {
		fmt.Fprintf(&b, "• %q\n", file)
	}
	statLines := strings.Split(strings.TrimSpace(p.Stat()), "\n")
	fmt.Fprintf(&b, "\n%s", safe(statLines[len(statLines)-1]))
	return b.String()
}
func (m *screen) repositoryReview() (string, bool) {
	switch mode := m.mode.(type) {
	case commitEditing:
		return commitReview(mode.plan), true
	case commitConfirming:
		return "提交说明：" + safe(mode.message) + "\n\n" + commitReview(mode.plan), true
	case pushConfirming:
		t := mode.plan.Target()
		return fmt.Sprintf("仓库：%s\n分支：%s\n远程：%s\n目标：%s\n地址：%s\n\n待推送：%d 个提交\n提交：%s\n\n普通推送，保留 pre-push 钩子。", safe(mode.plan.Path()), safe(t.Branch), safe(t.Remote), safe(t.RemoteRef), safe(mode.plan.URL()), t.Ahead, safe(t.LocalOID)), true
	case repositoryError:
		return actionTitle(mode.operation) + "\n\n" + safe(mode.detail), true
	}
	return "", false
}
