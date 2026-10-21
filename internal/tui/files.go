package tui

import (
	"context"
	"fmt"
	"path/filepath"
	"strings"
	"unicode"

	tea "charm.land/bubbletea/v2"
	"github.com/charmbracelet/x/ansi"
	"gw/internal/repoaction"
)

type fileArea uint8

const (
	worktreeFiles fileArea = iota
	indexFiles
)

type fileSession struct {
	snapshot repoaction.Files
	area     fileArea
	cursor   int
	selected map[string]struct{}
}
type filesBrowsing struct{ session fileSession }
type fileDiff struct {
	session fileSession
	content string
}
type fileError struct {
	session fileSession
	detail  string
}

func (filesBrowsing) interaction() {}
func (fileDiff) interaction()      {}
func (fileError) interaction()     {}

type filesLoaded struct {
	snapshot repoaction.Files
	area     fileArea
}
type filesFailed struct {
	origin interaction
	err    error
}
type diffLoaded struct {
	session fileSession
	content string
}

func fileContext(mode interaction) (fileSession, bool) {
	switch mode := mode.(type) {
	case filesBrowsing:
		return mode.session, true
	case fileDiff:
		return mode.session, true
	case fileError:
		return mode.session, true
	}
	return fileSession{}, false
}
func (s fileSession) rows() []repoaction.Change {
	rows := []repoaction.Change{}
	for _, c := range s.snapshot.Changes() {
		if (s.area == indexFiles && c.Area() == repoaction.Staged) || (s.area == worktreeFiles && c.Area() != repoaction.Staged) {
			rows = append(rows, c)
		}
	}
	return rows
}
func (m *screen) readFiles(path string, area fileArea) tea.Cmd {
	if path == "" {
		m.message = "请先选择仓库"
		return nil
	}
	origin := m.mode
	return m.startTask(stageSelected, func(ctx context.Context) tea.Msg {
		f, e := m.backend.ReadFiles(ctx, path)
		if e != nil {
			return filesFailed{origin, e}
		}
		return filesLoaded{f, area}
	})
}
func (m *screen) editFiles(s fileSession) tea.Cmd {
	rows := s.rows()
	selection := []repoaction.Change{}
	for _, c := range rows {
		if _, ok := s.selected[c.Key()]; ok {
			selection = append(selection, c)
		}
	}
	if len(selection) == 0 && len(rows) > 0 {
		selection = append(selection, rows[s.cursor])
	}
	if len(selection) == 0 {
		m.message = "当前区域没有可操作的文件"
		return nil
	}
	op := repoaction.Stage
	a := stageSelected
	if s.area == indexFiles {
		op = repoaction.Unstage
		a = unstageSelected
	}
	return m.startTask(a, func(ctx context.Context) tea.Msg {
		f, e := m.backend.EditIndex(ctx, s.snapshot, op, selection)
		if e != nil {
			return filesFailed{filesBrowsing{s}, e}
		}
		return filesLoaded{f, s.area}
	})
}
func (m *screen) showFileDiff(s fileSession) tea.Cmd {
	rows := s.rows()
	if len(rows) == 0 {
		m.message = "当前区域没有文件"
		return nil
	}
	change := rows[s.cursor]
	return m.startTask(diffSelected, func(ctx context.Context) tea.Msg {
		content, e := m.backend.FileDiff(ctx, s.snapshot, change)
		if e != nil {
			return filesFailed{filesBrowsing{s}, e}
		}
		return diffLoaded{s, fmt.Sprintf("%s\n\n%s", change.Display(), safeMultiline(content))}
	})
}
func safeMultiline(text string) string {
	return strings.Map(func(r rune) rune {
		if unicode.IsControl(r) && r != '\n' && r != '\t' {
			return ' '
		}
		return r
	}, ansi.Strip(text))
}
func (m *screen) updateFiles(msg tea.Msg) (tea.Cmd, bool) {
	switch msg := msg.(type) {
	case filesLoaded:
		m.mode = filesBrowsing{fileSession{snapshot: msg.snapshot, area: msg.area, selected: map[string]struct{}{}}}
		m.message = ""
		m.updateContent()
		return nil, true
	case diffLoaded:
		m.mode = fileDiff{msg.session, msg.content}
		m.message = ""
		m.updateContent()
		return nil, true
	case filesFailed:
		if s, ok := fileContext(msg.origin); ok {
			m.mode = fileError{s, msg.err.Error()}
		} else {
			m.mode = repositoryError{stageSelected, msg.err.Error()}
		}
		m.message = ""
		m.updateContent()
		return nil, true
	case tea.KeyPressMsg:
		s, ok := fileContext(m.mode)
		if !ok {
			return nil, false
		}
		key := msg.String()
		if key == ":" {
			return m.openCommand(), true
		}
		if key == "ctrl+c" || key == "q" {
			return tea.Quit, true
		}
		switch m.mode.(type) {
		case fileDiff, fileError:
			if key == "esc" {
				m.mode = filesBrowsing{s}
				m.updateContent()
				return nil, true
			}
			var cmd tea.Cmd
			m.viewport, cmd = m.viewport.Update(msg)
			return cmd, true
		}
		rows := s.rows()
		switch key {
		case "esc":
			m.mode = browsing{}
			m.message = ""
			m.updateContent()
			return nil, true
		case "tab":
			if s.area == worktreeFiles {
				s.area = indexFiles
			} else {
				s.area = worktreeFiles
			}
			s.cursor = 0
			s.selected = map[string]struct{}{}
		case "up", "k":
			s.cursor = max(0, s.cursor-1)
		case "down", "j":
			s.cursor = min(max(0, len(rows)-1), s.cursor+1)
		case "pgdown":
			s.cursor = min(max(0, len(rows)-1), s.cursor+max(1, m.bodyHeight()-5))
		case "pgup":
			s.cursor = max(0, s.cursor-max(1, m.bodyHeight()-5))
		case "space":
			if len(rows) > 0 {
				key := rows[s.cursor].Key()
				if _, ok := s.selected[key]; ok {
					delete(s.selected, key)
				} else {
					s.selected[key] = struct{}{}
				}
			}
		case "a":
			if len(s.selected) == len(rows) {
				s.selected = map[string]struct{}{}
			} else {
				for _, c := range rows {
					s.selected[c.Key()] = struct{}{}
				}
			}
		case "enter":
			return m.showFileDiff(s), true
		case "r":
			return m.readFiles(s.snapshot.Path(), s.area), true
		default:
			return nil, true
		}
		m.mode = filesBrowsing{s}
		return nil, true
	}
	return nil, false
}
func (m *screen) filesView(s fileSession) string {
	title := "工作区"
	if s.area == indexFiles {
		title = "暂存区"
	}
	rows := s.rows()
	lines := []string{fmt.Sprintf("%s · %d 个文件 · 已选 %d", title, len(rows), len(s.selected)), ""}
	capacity := max(1, m.bodyHeight()-7)
	start := max(0, s.cursor-capacity+1)
	for i := start; i < min(len(rows), start+capacity); i++ {
		c := rows[i]
		box := "[ ]"
		if _, ok := s.selected[c.Key()]; ok {
			box = "[✓]"
		}
		line := fmt.Sprintf("  %s %c %s", box, c.Kind(), safe(c.Display()))
		if i == s.cursor {
			line = selectedStyle.Render("› " + strings.TrimPrefix(line, "  "))
		}
		lines = append(lines, ansi.Truncate(line, max(1, m.width-6), "…"))
	}
	if len(rows) == 0 {
		lines = append(lines, "此区域没有变更")
	}
	return frame(safe(filepath.Base(s.snapshot.Path()))+" · "+safe(s.snapshot.Branch()), strings.Join(lines, "\n"), m.width-2, m.bodyHeight())
}
