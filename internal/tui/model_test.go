package tui

import (
	"context"
	"errors"
	"fmt"
	"strings"
	"testing"
	"time"

	tea "charm.land/bubbletea/v2"
	"charm.land/lipgloss/v2"
	"gw/internal/app"
	"gw/internal/model"
	"gw/internal/scheduler"
	"gw/internal/state"
)

type noGit struct{ t *testing.T }

func (g noGit) Run(context.Context, string, ...string) ([]byte, error) {
	g.t.Error("浏览或配置不应调用 Git")
	return nil, errors.New("unexpected Git")
}

type scheduleReader struct{}

func (scheduleReader) Registration(context.Context) (scheduler.Registration, error) {
	return scheduler.Loaded, nil
}
func (scheduleReader) Schedule() (scheduler.Schedule, error) {
	return scheduler.Saved{Interval: 21600}, nil
}

type backend struct {
	*app.Service
	run func(context.Context, func(app.Event)) (model.Report, error)
}

func (b backend) Run(ctx context.Context, emit func(app.Event)) (model.Report, error) {
	return b.run(ctx, emit)
}
func fixture(t *testing.T) (*screen, *app.Service) {
	t.Helper()
	s := app.New(t.TempDir(), scheduleReader{})
	s.Git = noGit{t}
	c := state.Config{Interval: 21600, PushPolicy: model.FastForward, Repositories: []string{"/path/to/clean", "/path/to/dirty", "/path/to/new"}, Excluded: []string{}}
	if e := s.Store.WriteConfig(c); e != nil {
		t.Fatal(e)
	}
	lock, e := s.Store.Lock()
	if e != nil {
		t.Fatal(e)
	}
	state.Unlock(lock)
	r := model.Report{SchemaVersion: 3, PushPolicy: model.FastForward, Status: model.Attention, StartedAt: time.Now().Format(time.RFC3339Nano), FinishedAt: time.Now().Format(time.RFC3339Nano), Repositories: []model.Repository{
		model.Normal(c.Repositories[0], []model.BranchWithoutUpstream{{Branch: "experiment", Reason: model.NotConfigured}}, nil).Record(),
		model.NeedsAttention(c.Repositories[1], model.Issue{Kind: model.Dirty, Detail: "存在未提交文件"}, nil, nil, nil).Record(),
	}}
	if e = s.Store.WriteReport(r); e != nil {
		t.Fatal(e)
	}
	b := backend{s, func(context.Context, func(app.Event)) (model.Report, error) {
		t.Error("浏览不能检查")
		return model.Report{}, nil
	}}
	m := newScreen(context.Background(), b, func(context.Context) error { t.Error("不应应用调度"); return nil })
	m.dispatch = func(tea.Msg) {}
	m.Update(m.Init()())
	m.Update(tea.WindowSizeMsg{Width: 100, Height: 30})
	return m, s
}
func press(m *screen, key string) tea.Cmd {
	code := []rune(key)[0]
	switch key {
	case "enter":
		code = tea.KeyEnter
	case "esc":
		code = tea.KeyEscape
	case "tab":
		code = tea.KeyTab
	case "down":
		code = tea.KeyDown
	case "up":
		code = tea.KeyUp
	}
	_, cmd := m.Update(tea.KeyPressMsg{Code: code})
	return cmd
}
func settle(m *screen, cmd tea.Cmd) {
	if cmd == nil {
		return
	}
	_, next := m.Update(cmd())
	if next != nil {
		m.Update(next())
	}
}
func TestBrowseUsesCachedReportAndExplicitRegistry(t *testing.T) {
	m, _ := fixture(t)
	if len(m.rows()) != 3 || m.rows()[2].Status != "" {
		t.Fatal(m.rows())
	}
	press(m, "2")
	if len(m.rows()) != 1 || m.rows()[0].Path != "/path/to/dirty" {
		t.Fatal(m.rows())
	}
	press(m, "3")
	if len(m.rows()) != 1 || len(m.rows()[0].Branches) != 1 {
		t.Fatal(m.rows())
	}
	press(m, "4")
	if len(m.rows()) != 0 {
		t.Fatal(m.rows())
	}
	press(m, "1")
	m.prompt(search, "dirty")
	press(m, "enter")
	if len(m.rows()) != 1 {
		t.Fatal(m.rows())
	}
	press(m, "enter")
	if m.page != detail || !strings.Contains(m.View().Content, "存在未提交文件") {
		t.Fatal(m.View().Content)
	}
	press(m, "esc")
	settle(m, press(m, "r"))
	if m.query != "" || len(m.rows()) != 3 {
		t.Fatal(m.rows())
	}
}
func TestConfigurationRequiresConfirmationAndDoesNotCheck(t *testing.T) {
	m, s := fixture(t)
	press(m, "tab")
	m.prompt(interval, "8h")
	press(m, "enter")
	c, _ := s.Config()
	if c.Interval != 21600 {
		t.Fatal(c)
	}
	press(m, "n")
	c, _ = s.Config()
	if c.Interval != 21600 {
		t.Fatal(c)
	}
	m.prompt(interval, "8h")
	press(m, "enter")
	settle(m, press(m, "y"))
	c, _ = s.Config()
	if c.Interval != 28800 || !strings.Contains(m.message, "应用调度") {
		t.Fatal(c, m.message)
	}
	m.prompt(policy, "disabled")
	press(m, "enter")
	settle(m, press(m, "y"))
	c, _ = s.Config()
	if c.PushPolicy != model.Disabled {
		t.Fatal(c)
	}
	press(m, "tab")
	press(m, "d")
	press(m, "n")
	c, _ = s.Config()
	if len(c.Repositories) != 3 {
		t.Fatal(c)
	}
	press(m, "d")
	settle(m, press(m, "y"))
	c, _ = s.Config()
	if len(c.Repositories) != 2 {
		t.Fatal(c)
	}
}
func TestCheckConfirmationProgressAndCancellation(t *testing.T) {
	m, s := fixture(t)
	started := make(chan struct{})
	events := make(chan tea.Msg, 4)
	m.dispatch = func(msg tea.Msg) { events <- msg }
	m.backend = backend{s, func(ctx context.Context, emit func(app.Event)) (model.Report, error) {
		close(started)
		emit(app.Checking{Path: "/path/to/clean"})
		<-ctx.Done()
		return model.Report{}, ctx.Err()
	}}
	press(m, "c")
	press(m, "n")
	select {
	case <-started:
		t.Fatal("取消确认不应检查")
	default:
	}
	press(m, "c")
	cmd := press(m, "y")
	<-started
	m.Update(<-events)
	if !strings.Contains(m.message, "正在检查") {
		t.Fatal(m.message)
	}
	press(m, "esc")
	if _, ok := m.mode.(stopping); !ok {
		t.Fatal(m.mode)
	}
	settle(m, cmd)
	m.workers.Wait()
	if !strings.Contains(m.message, "context canceled") {
		t.Fatal(m.message)
	}
	r, e := s.Report()
	if e != nil || len(r.Repositories) != 2 {
		t.Fatal(r, e)
	}
}
func TestFailedOperationRemainsVisibleAfterRefresh(t *testing.T) {
	m, s := fixture(t)
	m.backend = backend{s, func(context.Context, func(app.Event)) (model.Report, error) {
		return model.Report{}, errors.New("远程连接失败")
	}}
	press(m, "c")
	settle(m, press(m, "y"))
	if !strings.Contains(m.message, "远程连接失败") {
		t.Fatal(m.message)
	}
}
func TestValidationKeepsEditingAndDoesNotMutate(t *testing.T) {
	m, s := fixture(t)
	for _, item := range []struct {
		a     action
		value string
	}{{add, "relative/path"}, {interval, "1s"}, {policy, "enabled"}} {
		m.prompt(item.a, item.value)
		press(m, "enter")
		if _, ok := m.mode.(editing); !ok {
			t.Fatal(m.mode)
		}
	}
	c, _ := s.Config()
	if c.Interval != 21600 || c.PushPolicy != model.FastForward {
		t.Fatal(c)
	}
}

func TestSearchReceivesTypedTextAndNavigationKeysStayInInput(t *testing.T) {
	m, _ := fixture(t)
	press(m, "/")
	for _, r := range "dirty" {
		m.Update(tea.KeyPressMsg{Code: r, Text: string(r)})
	}
	mode := m.mode.(editing)
	if !mode.input.Focused() || mode.input.Value() != "dirty" {
		t.Fatal(mode.input.Value(), mode.input.Focused())
	}
	press(m, "enter")
	if m.query != "dirty" || len(m.rows()) != 1 {
		t.Fatal(m.query, m.rows())
	}
}
func TestResizeScrollAndUntrustedTerminalText(t *testing.T) {
	m, _ := fixture(t)
	m.page = help
	m.updateContent()
	m.viewport.GotoBottom()
	if m.viewport.YOffset() == 0 {
		t.Fatal("帮助应可滚动")
	}
	m.Update(tea.WindowSizeMsg{Width: 60, Height: 18})
	v := m.View()
	if !v.AltScreen || !strings.Contains(v.Content, "gw") {
		t.Fatal(v)
	}
	m.Update(tea.WindowSizeMsg{Width: 40, Height: 10})
	if !strings.Contains(m.View().Content, "调整") {
		t.Fatal(m.View())
	}
	injected := "repo\x1b[2J\x1b]52;c;secret\a\rname"
	if got := safe(injected); strings.ContainsAny(got, "\x1b\a\r") {
		t.Fatalf("unsafe: %q", got)
	}
}
func TestApplyScheduleIsSeparateFromSavingFrequency(t *testing.T) {
	m, _ := fixture(t)
	calls := 0
	m.apply = func(context.Context) error { calls++; return nil }
	press(m, "tab")
	press(m, "l")
	if calls != 0 {
		t.Fatal(calls)
	}
	settle(m, press(m, "y"))
	if calls != 1 {
		t.Fatal(calls)
	}
}
func TestLongListKeepsSelectionVisible(t *testing.T) {
	m, _ := fixture(t)
	m.config.Repositories = nil
	for i := 0; i < 80; i++ {
		m.config.Repositories = append(m.config.Repositories, fmt.Sprintf("/path/to/repo%02d", i))
	}
	m.selected = 79
	view := m.listView()
	if !strings.Contains(view, "repo79") || strings.Contains(view, "repo00") {
		t.Fatal(view)
	}
}

func TestViewsFitSupportedTerminalSizes(t *testing.T) {
	m, _ := fixture(t)
	for _, size := range []tea.WindowSizeMsg{{Width: 60, Height: 18}, {Width: 80, Height: 24}, {Width: 110, Height: 30}} {
		m.Update(size)
		for _, p := range []page{repositories, detail, settings, help} {
			m.page = p
			m.mode = browsing{}
			m.updateContent()
			view := m.View().Content
			if lipgloss.Height(view) > size.Height || lipgloss.Width(view) > size.Width {
				t.Fatalf("page %d size %+v produced %dx%d", p, size, lipgloss.Width(view), lipgloss.Height(view))
			}
		}
		m.prompt(add, "/path/to/repository")
		view := m.View().Content
		if lipgloss.Height(view) > size.Height || lipgloss.Width(view) > size.Width {
			t.Fatalf("input size %+v produced %dx%d", size, lipgloss.Width(view), lipgloss.Height(view))
		}
	}
}
