package tui

import (
	"path/filepath"
	"strings"

	"charm.land/bubbles/v2/textinput"
	tea "charm.land/bubbletea/v2"
	"github.com/charmbracelet/x/ansi"
)

type commandEditing struct {
	input    textinput.Model
	origin   interaction
	path     string
	selected int
}

func (commandEditing) interaction() {}

type commandScope uint8

const (
	repositoryScope commandScope = iota
	globalScope
	viewScope
)

type commandSpec struct {
	name, description string
	scope             commandScope
}

var commands = []commandSpec{
	{"stage", "选择工作区文件并暂存", repositoryScope},
	{"unstage", "选择暂存区文件并取消暂存", repositoryScope},
	{"diff", "查看文件差异", repositoryScope},
	{"commit", "预览暂存内容，输入提交说明", repositoryScope},
	{"push", "核对当前分支上游并确认推送", repositoryScope},
	{"check all", "检查全部注册仓库，按配置自动推送", globalScope},
	{"refresh", "重新读取报告；文件面板中刷新文件", viewScope},
	{"settings", "打开设置", globalScope},
	{"add", "添加仓库", globalScope}, {"remove", "移除当前仓库的注册项", repositoryScope},
	{"exclude", "排除目录", globalScope}, {"unexclude", "取消排除目录", globalScope},
	{"scan", "扫描预览", globalScope}, {"import", "扫描并导入", globalScope},
	{"interval", "设置检查间隔", globalScope}, {"autopush", "设置自动推送策略", globalScope},
	{"apply", "应用系统调度", globalScope}, {"help", "查看帮助", globalScope}, {"quit", "退出", globalScope},
}

func commandMatches(value string) []commandSpec {
	result := []commandSpec{}
	for _, c := range commands {
		if strings.HasPrefix(c.name, strings.TrimSpace(value)) {
			result = append(result, c)
		}
	}
	return result
}
func (m *screen) openCommand() tea.Cmd {
	path := m.selectedPath()
	if session, ok := fileContext(m.mode); ok {
		path = session.snapshot.Path()
	}
	input := textinput.New()
	input.Prompt = ": "
	input.SetWidth(max(1, m.width-6))
	input.SetVirtualCursor(true)
	cmd := input.Focus()
	m.mode = commandEditing{input: input, origin: m.mode, path: path}
	m.message = ""
	return cmd
}
func (m *screen) updateCommand(msg tea.Msg) (tea.Cmd, bool) {
	mode, ok := m.mode.(commandEditing)
	if !ok {
		return nil, false
	}
	if _, ok := msg.(tea.WindowSizeMsg); ok {
		return nil, false
	}
	switch msg := msg.(type) {
	case loaded, reloaded, progress:
		return nil, false
	case tea.KeyPressMsg:
		matches := commandMatches(mode.input.Value())
		switch msg.String() {
		case "esc":
			m.mode = mode.origin
			m.updateContent()
			return nil, true
		case "ctrl+c":
			return tea.Quit, true
		case "up":
			mode.selected = max(0, mode.selected-1)
		case "down":
			mode.selected = min(max(0, len(matches)-1), mode.selected+1)
		case "tab":
			if len(matches) > 0 {
				mode.input.SetValue(matches[min(mode.selected, len(matches)-1)].name)
				mode.input.CursorEnd()
				mode.selected = 0
			}
		case "enter":
			value := strings.TrimSpace(mode.input.Value())
			for _, c := range commands {
				if c.name == value {
					m.mode = mode.origin
					m.message = ""
					return m.runCommand(c.name, mode.path), true
				}
			}
			m.message = "未知命令；按 Tab 补全后再执行"
			return nil, true
		default:
			var cmd tea.Cmd
			mode.input, cmd = mode.input.Update(msg)
			mode.selected = 0
			m.mode = mode
			return cmd, true
		}
		m.mode = mode
		return nil, true
	default:
		var cmd tea.Cmd
		mode.input, cmd = mode.input.Update(msg)
		m.mode = mode
		return cmd, true
	}
}
func (m *screen) runCommand(name, path string) tea.Cmd {
	switch name {
	case "stage", "unstage", "diff":
		a := stageSelected
		if name == "unstage" {
			a = unstageSelected
		}
		if name == "diff" {
			a = diffSelected
		}
		if session, ok := fileContext(m.mode); ok {
			if a == diffSelected {
				return m.showFileDiff(session)
			}
			if a == stageSelected && session.area == worktreeFiles {
				return m.editFiles(session)
			}
			if a == unstageSelected && session.area == indexFiles {
				return m.editFiles(session)
			}
		}
		area := worktreeFiles
		if a == unstageSelected {
			area = indexFiles
		}
		return m.readFiles(path, area)
	case "commit":
		return m.prepareRepositoryAt(commitSelected, path)
	case "push":
		return m.prepareRepositoryAt(pushSelected, path)
	case "remove":
		if path == "" {
			m.message = "请先选择仓库"
			return nil
		}
		m.mode = confirming{remove, path}
		return nil
	case "refresh":
		if session, ok := fileContext(m.mode); ok {
			return m.readFiles(session.snapshot.Path(), session.area)
		}
		return m.load()
	case "settings":
		m.mode = browsing{}
		m.page = settings
		m.updateContent()
		return nil
	case "help":
		m.mode = browsing{}
		m.page = help
		m.updateContent()
		return nil
	case "quit":
		return tea.Quit
	}
	actions := map[string]action{"check all": check, "add": add, "exclude": exclude, "unexclude": unexclude, "scan": scan, "import": importTree, "interval": interval, "autopush": policy, "apply": applySchedule}
	a, ok := actions[name]
	if !ok {
		panic("unknown command")
	}
	return m.activate(a)
}
func (m *screen) commandView(mode commandEditing) string {
	matches := commandMatches(mode.input.Value())
	capacity := max(1, m.bodyHeight()-5)
	start := max(0, mode.selected-capacity+1)
	lines := []string{}
	for i := start; i < min(len(matches), start+capacity); i++ {
		c := matches[i]
		scope := "全局"
		if c.scope == repositoryScope {
			scope = "当前仓库"
		}
		if c.scope == viewScope {
			scope = "当前视图"
		}
		line := "  " + cell(c.name, 12) + " " + scope + " · " + c.description
		if i == mode.selected {
			line = selectedStyle.Render("› " + strings.TrimPrefix(line, "  "))
		}
		lines = append(lines, ansi.Truncate(line, max(1, m.width-6), "…"))
	}
	if len(matches) == 0 {
		lines = append(lines, "没有匹配命令")
	}
	context := "未选择仓库"
	if mode.path != "" {
		context = safe(filepath.Base(mode.path))
	}
	return frame("命令 · "+context, strings.Join(lines, "\n"), m.width-2, m.bodyHeight())
}
