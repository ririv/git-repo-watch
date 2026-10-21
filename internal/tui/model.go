// Package tui adapts application services to an interactive terminal interface.
package tui

import (
	"context"
	"fmt"
	"io"
	"os"
	"strings"
	"sync"

	"charm.land/bubbles/v2/textinput"
	"charm.land/bubbles/v2/viewport"
	tea "charm.land/bubbletea/v2"
	"github.com/charmbracelet/x/term"
	"gw/internal/app"
	"gw/internal/discovery"
	"gw/internal/model"
	"gw/internal/state"
)

type Backend interface {
	Snapshot(context.Context) (app.Snapshot, error)
	Config() (state.Config, error)
	Run(context.Context, func(app.Event)) (model.Report, error)
	Add(context.Context, []string) (int, error)
	Remove(string) error
	Exclude(string) error
	Unexclude(string) error
	Interval(int) error
	Policy(model.PushPolicy) error
	Scan(context.Context, string, app.ScanMode) (discovery.Report, error)
}

type page uint8

const (
	repositories page = iota
	detail
	settings
	help
)

type action uint8

const (
	search action = iota
	add
	remove
	exclude
	unexclude
	scan
	importTree
	interval
	policy
	check
	applySchedule
)

// Interaction is a closed union; a modal cannot be editing and executing at once.
type interaction interface{ interaction() }
type browsing struct{}
type editing struct {
	action action
	input  textinput.Model
}
type confirming struct {
	action action
	value  string
}
type working struct {
	action action
	cancel context.CancelFunc
}
type stopping struct{}

func (browsing) interaction()   {}
func (editing) interaction()    {}
func (confirming) interaction() {}
func (working) interaction()    {}
func (stopping) interaction()   {}

type loaded struct {
	snapshot app.Snapshot
	config   state.Config
	err      error
}
type completed struct {
	message string
	err     error
}
type progress struct{ event app.Event }

type screen struct {
	ctx           context.Context
	backend       Backend
	apply         func(context.Context) error
	dispatch      func(tea.Msg)
	workers       *sync.WaitGroup
	snapshot      app.Snapshot
	config        state.Config
	page          page
	mode          interaction
	filter        app.Filter
	query         string
	selected      int
	width, height int
	viewport      viewport.Model
	message       string
}

func newScreen(ctx context.Context, b Backend, apply func(context.Context) error) *screen {
	v := viewport.New()
	v.SoftWrap = true
	return &screen{ctx: ctx, backend: b, apply: apply, workers: &sync.WaitGroup{}, mode: browsing{}, filter: app.All, width: 80, height: 24, viewport: v, message: "正在读取已有报告…"}
}
func (m *screen) load() tea.Cmd {
	return func() tea.Msg {
		s, e := m.backend.Snapshot(m.ctx)
		if e != nil {
			return loaded{err: e}
		}
		c, e := m.backend.Config()
		return loaded{s, c, e}
	}
}
func (m *screen) Init() tea.Cmd { return m.load() }
func (m *screen) rows() []model.Repository {
	rows := []model.Repository{}
	byPath := map[string]model.Repository{}
	if saved, ok := m.snapshot.Report.(app.AvailableReport); ok {
		for _, r := range app.FilterReport(saved.Report, m.filter).Repositories {
			byPath[r.Path] = r
		}
	}
	for _, path := range m.config.Repositories {
		r, ok := byPath[path]
		if !ok && m.filter != app.All {
			continue
		}
		if !ok {
			r = model.Repository{Path: path}
		}
		if strings.Contains(strings.ToLower(path), strings.ToLower(m.query)) {
			rows = append(rows, r)
		}
	}
	return rows
}
func (m *screen) selection() (model.Repository, bool) {
	rows := m.rows()
	if m.selected < 0 || m.selected >= len(rows) {
		return model.Repository{}, false
	}
	return rows[m.selected], true
}
func (m *screen) clamp() { m.selected = max(0, min(m.selected, len(m.rows())-1)) }
func (m *screen) prompt(a action, value string) tea.Cmd {
	input := textinput.New()
	input.Prompt = "› "
	input.SetValue(value)
	input.SetWidth(max(20, m.width-6))
	input.SetVirtualCursor(true)
	cmd := input.Focus()
	m.mode = editing{a, input}
	return cmd
}
func (m *screen) execute(a action, value string) tea.Cmd {
	ctx, cancel := context.WithCancel(m.ctx)
	m.mode = working{a, cancel}
	m.message = actionTitle(a) + "…"
	result := make(chan completed, 1)
	m.workers.Add(1)
	go func() {
		defer m.workers.Done()
		defer cancel()
		message, e := perform(ctx, m.backend, m.apply, a, value, func(e app.Event) { m.dispatch(progress{e}) })
		result <- completed{message, e}
	}()
	return func() tea.Msg { return <-result }
}
func (m *screen) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case loaded:
		if msg.err != nil {
			m.message = "读取失败：" + msg.err.Error() + "；当前内容未更新"
			return m, nil
		}
		m.snapshot, m.config = msg.snapshot, msg.config
		m.message = "已读取本机配置与已有报告"
		m.clamp()
		m.updateContent()
		return m, nil
	case completed:
		m.mode = browsing{}
		if msg.err != nil {
			m.message = "操作失败：" + msg.err.Error()
		} else {
			m.message = msg.message
		}
		// 刷新消息与操作结果分开保留，避免成功刷新掩盖实际错误。
		result := m.message
		return m, func() tea.Msg { v := m.load()().(loaded); return reloaded{v, result} }
	case reloaded:
		if msg.data.err != nil {
			m.message = msg.result + "；刷新失败：" + msg.data.err.Error()
		} else {
			m.snapshot, m.config = msg.data.snapshot, msg.data.config
			m.message = msg.result
			m.clamp()
			m.updateContent()
		}
		return m, nil
	case progress:
		switch e := msg.event.(type) {
		case app.Checking:
			m.message = "正在检查：" + e.Path
		case app.Checked:
			m.message = "已检查：" + e.Repository.Path
		case app.NotificationProblem:
			m.message = "系统通知发送失败：" + e.Notice.Error
		}
		return m, nil
	case tea.WindowSizeMsg:
		m.width, m.height = msg.Width, msg.Height
		if mode, ok := m.mode.(editing); ok {
			mode.input.SetWidth(max(1, m.width-6))
			m.mode = mode
		}
		m.updateContent()
		return m, nil
	case tea.KeyPressMsg:
		key := msg.String()
		switch mode := m.mode.(type) {
		case working:
			if key == "esc" || key == "ctrl+c" {
				mode.cancel()
				m.mode = stopping{}
				m.message = "正在取消，等待 Git 子进程退出…"
			}
			return m, nil
		case stopping:
			return m, nil
		case editing:
			if key == "esc" {
				m.mode = browsing{}
				return m, nil
			}
			if key == "ctrl+c" {
				return m, tea.Quit
			}
			if key == "enter" {
				value := mode.input.Value()
				if mode.action == search {
					m.query = value
					m.selected = 0
					m.mode = browsing{}
					return m, nil
				}
				if e := validate(mode.action, value); e != nil {
					m.message = e.Error()
					return m, nil
				}
				m.mode = confirming{mode.action, value}
				return m, nil
			}
			var cmd tea.Cmd
			mode.input, cmd = mode.input.Update(msg)
			m.mode = mode
			return m, cmd
		case confirming:
			if key == "y" {
				return m, m.execute(mode.action, mode.value)
			}
			if key == "esc" || key == "n" {
				m.mode = browsing{}
				m.message = "已取消"
			}
			return m, nil
		}
		switch key {
		case "q", "ctrl+c":
			return m, tea.Quit
		case "?":
			m.page = help
			m.updateContent()
			return m, nil
		case "esc":
			m.page = repositories
			m.query = ""
			m.selected = 0
			m.updateContent()
			return m, nil
		case "tab":
			if m.page == settings {
				m.page = repositories
			} else {
				m.page = settings
			}
			m.updateContent()
			return m, nil
		case "r":
			return m, m.load()
		case "c":
			m.mode = confirming{check, ""}
			return m, nil
		}
		if m.page == repositories {
			switch key {
			case "1", "2", "3", "4":
				m.filter = map[string]app.Filter{"1": app.All, "2": app.Problems, "3": app.WithoutUpstream, "4": app.Pushes}[key]
				m.selected = 0
			case "/":
				return m, m.prompt(search, m.query)
			case "up", "k":
				m.selected--
				m.clamp()
			case "down", "j":
				m.selected++
				m.clamp()
			case "pgdown":
				m.selected += max(1, m.height-12)
				m.clamp()
			case "pgup":
				m.selected -= max(1, m.height-12)
				m.clamp()
			case "home", "g":
				m.selected = 0
			case "end", "G":
				m.selected = max(0, len(m.rows())-1)
			case "enter":
				if _, ok := m.selection(); ok {
					m.page = detail
					m.updateContent()
				}
			case "a":
				return m, m.prompt(add, "")
			case "d":
				if r, ok := m.selection(); ok {
					m.mode = confirming{remove, r.Path}
				}
			}
			return m, nil
		}
		if m.page == settings {
			switch key {
			case "a":
				return m, m.prompt(add, "")
			case "x":
				return m, m.prompt(exclude, "")
			case "u":
				return m, m.prompt(unexclude, "")
			case "s":
				return m, m.prompt(scan, "")
			case "i":
				return m, m.prompt(importTree, "")
			case "t":
				return m, m.prompt(interval, fmt.Sprintf("%ds", m.config.Interval))
			case "p":
				return m, m.prompt(policy, string(m.config.PushPolicy))
			case "l":
				m.mode = confirming{applySchedule, ""}
				return m, nil
			}
		}
	}
	if mode, ok := m.mode.(editing); ok {
		var cmd tea.Cmd
		mode.input, cmd = mode.input.Update(msg)
		m.mode = mode
		return m, cmd
	}
	var cmd tea.Cmd
	m.viewport, cmd = m.viewport.Update(msg)
	return m, cmd
}

type reloaded struct {
	data   loaded
	result string
}

// Run requires an interactive terminal and restores it before returning.
func Run(ctx context.Context, b Backend, apply func(context.Context) error, input *os.File, out io.Writer) error {
	output, ok := out.(*os.File)
	if !term.IsTerminal(input.Fd()) || !ok || !term.IsTerminal(output.Fd()) {
		return fmt.Errorf("gw tui 需要交互式终端；请使用 gw status 或 gw report 查询")
	}
	ctx, cancel := context.WithCancel(ctx)
	defer cancel()
	m := newScreen(ctx, b, apply)
	p := tea.NewProgram(m, tea.WithContext(ctx), tea.WithInput(input), tea.WithOutput(out))
	m.dispatch = p.Send
	_, e := p.Run()
	cancel()
	m.workers.Wait()
	return e
}
