package tui

import (
	tea "charm.land/bubbletea/v2"
	"gw/internal/app"
)

func (m *screen) selectedPath() string {
	if r, ok := m.selection(); ok {
		return r.Path
	}
	return ""
}
func (m *screen) navigate(msg tea.KeyPressMsg) (tea.Model, tea.Cmd) {
	key := msg.String()
	switch key {
	case ":":
		return m, m.openCommand()
	case "C", "P":
		if m.page == repositories || m.page == detail {
			a := commitSelected
			if key == "P" {
				a = pushSelected
			}
			return m, m.activate(a)
		}
	case "q", "ctrl+c":
		return m, tea.Quit
	case "?":
		if m.page != help {
			m.history = append(m.history, m.page)
		}
		m.page = help
		m.updateContent()
		return m, nil
	case "esc":
		if len(m.history) > 0 {
			m.page = m.history[len(m.history)-1]
			m.history = m.history[:len(m.history)-1]
		} else if m.page != repositories {
			m.page = repositories
		} else {
			path := m.selectedPath()
			m.query = ""
			m.focusPath(path)
		}
		m.updateContent()
		return m, nil
	case "space":
		m.mode = choosing{}
		return m, nil
	case "tab":
		m.history = nil
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
		return m, m.activate(check)
	}
	if m.page == repositories {
		switch key {
		case "1", "2", "3", "4":
			path := m.selectedPath()
			m.filter = map[string]app.Filter{"1": app.All, "2": app.Problems, "3": app.WithoutUpstream, "4": app.Pushes}[key]
			m.focusPath(path)
		case "o":
			path := m.selectedPath()
			if m.order == priorityOrder {
				m.order = pathOrder
			} else {
				m.order = priorityOrder
			}
			m.focusPath(path)
		case "/":
			return m, m.prompt(search, m.query)
		case "up", "k":
			m.selected--
			m.clamp()
		case "down", "j":
			m.selected++
			m.clamp()
		case "pgdown":
			m.selected += max(1, m.bodyHeight()-8)
			m.clamp()
		case "pgup":
			m.selected -= max(1, m.bodyHeight()-8)
			m.clamp()
		case "home", "g":
			m.selected = 0
		case "end", "G":
			m.selected = max(0, len(m.rows())-1)
		case "enter":
			if _, ok := m.selection(); ok {
				m.history = append(m.history, m.page)
				m.page = detail
				m.updateContent()
			}
		case "a":
			return m, m.activate(add)
		case "d":
			return m, m.activate(remove)
		}
		return m, nil
	}
	if m.page == settings {
		switch key {
		case "up", "k":
			m.settingSelected = max(0, m.settingSelected-1)
			m.updateContent()
			return m, nil
		case "down", "j":
			m.settingSelected = min(len(settingsItems)-1, m.settingSelected+1)
			m.updateContent()
			return m, nil
		case "enter":
			return m, m.activate(settingsItems[m.settingSelected].action)
		default:
			for _, item := range settingsItems {
				if key == item.key {
					return m, m.activate(item.action)
				}
			}
		}
	}
	var cmd tea.Cmd
	m.viewport, cmd = m.viewport.Update(msg)
	return m, cmd
}
