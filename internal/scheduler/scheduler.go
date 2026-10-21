// Package scheduler installs native binaries and manages the user's launchd registration.
package scheduler

import (
	"bytes"
	"context"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"encoding/xml"
	"fmt"
	"gw/internal/state"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
)

const Label = "local.git-repo-watch"

type Registration string

const (
	Loaded    Registration = "loaded"
	NotLoaded Registration = "not_loaded"
)

type Schedule interface{ schedule() }
type Missing struct{}
type Saved struct{ Interval int }

func (Missing) schedule()                    {}
func (Saved) schedule()                      {}
func (Missing) MarshalJSON() ([]byte, error) { return []byte(`{"status":"missing"}`), nil }
func (s Saved) MarshalJSON() ([]byte, error) {
	return json.Marshal(struct {
		Status   string `json:"status"`
		Interval int    `json:"interval_seconds"`
	}{"saved", s.Interval})
}

type Reader interface {
	Registration(context.Context) (Registration, error)
	Schedule() (Schedule, error)
}
type Manager struct {
	UserHome, ConfigHome string
	UID                  int
}

func New() (Manager, error) {
	home, e := os.UserHomeDir()
	if e != nil {
		return Manager{}, e
	}
	config, e := os.UserConfigDir()
	return Manager{home, config, os.Getuid()}, e
}
func (m Manager) Target() string { return fmt.Sprintf("gui/%d/%s", m.UID, Label) }
func (m Manager) Plist() string {
	return filepath.Join(m.UserHome, "Library", "LaunchAgents", Label+".plist")
}
func (m Manager) Binary() string   { return filepath.Join(m.UserHome, ".local", "bin", "gw") }
func (m Manager) Location() string { return filepath.Join(m.ConfigHome, "gw", "location.json") }
func command(ctx context.Context, args ...string) ([]byte, error) {
	data, e := exec.CommandContext(ctx, "/bin/launchctl", args...).CombinedOutput()
	if e != nil {
		return data, fmt.Errorf("launchctl %s：%w\n%s", strings.Join(args, " "), e, data)
	}
	return data, nil
}
func (m Manager) Registration(ctx context.Context) (Registration, error) {
	cmd := exec.CommandContext(ctx, "/bin/launchctl", "print", m.Target())
	data, e := cmd.CombinedOutput()
	if e == nil {
		return Loaded, nil
	}
	if exit, ok := e.(*exec.ExitError); ok && exit.ExitCode() == 113 {
		return NotLoaded, nil
	}
	return "", fmt.Errorf("无法查询系统任务：%w\n%s", e, data)
}
func (m Manager) Schedule() (Schedule, error) {
	data, e := os.ReadFile(m.Plist())
	if os.IsNotExist(e) {
		return Missing{}, nil
	}
	if e != nil {
		return nil, e
	}
	return ParseSchedule(data)
}
func ParseSchedule(data []byte) (Schedule, error) {
	decoder := xml.NewDecoder(bytes.NewReader(data))
	want := false
	for {
		token, e := decoder.Token()
		if e != nil {
			return nil, fmt.Errorf("无法读取系统任务间隔：%w", e)
		}
		start, ok := token.(xml.StartElement)
		if !ok {
			continue
		}
		if start.Name.Local == "key" {
			var key string
			if e = decoder.DecodeElement(&key, &start); e != nil {
				return nil, e
			}
			want = key == "StartInterval"
		} else if want && start.Name.Local == "integer" {
			var seconds int
			if e = decoder.DecodeElement(&seconds, &start); e != nil {
				return nil, e
			}
			if seconds < 60 {
				return nil, fmt.Errorf("系统任务间隔无效")
			}
			return Saved{seconds}, nil
		}
	}
}
func Plist(root, binary string, interval int) []byte {
	var out bytes.Buffer
	out.WriteString(xml.Header + `<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">` + "\n<plist version=\"1.0\"><dict>\n")
	text := func(tag, value string) {
		out.WriteString("<" + tag + ">")
		xml.EscapeText(&out, []byte(value))
		out.WriteString("</" + tag + ">\n")
	}
	pair := func(k, v string) { text("key", k); text("string", v) }
	pair("Label", Label)
	text("key", "ProgramArguments")
	out.WriteString("<array>")
	for _, v := range []string{binary, "--home", root, "check"} {
		text("string", v)
	}
	out.WriteString("</array>\n")
	pair("WorkingDirectory", root)
	text("key", "StartInterval")
	text("integer", fmt.Sprint(interval))
	text("key", "RunAtLoad")
	out.WriteString("<false/>\n")
	pair("ProcessType", "Background")
	text("key", "EnvironmentVariables")
	out.WriteString("<dict>")
	pair("PATH", "/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin")
	out.WriteString("</dict>")
	pair("StandardOutPath", filepath.Join(root, "var", "launchd.stdout.log"))
	pair("StandardErrorPath", filepath.Join(root, "var", "launchd.stderr.log"))
	out.WriteString("</dict></plist>\n")
	return out.Bytes()
}

type Location struct {
	Home       string `json:"home"`
	BinaryHash string `json:"binary_sha256"`
}

func (m Manager) ReadLocation() (Location, error) {
	var value Location
	data, e := os.ReadFile(m.Location())
	if e != nil {
		return value, e
	}
	if e = json.Unmarshal(data, &value); e != nil {
		return value, e
	}
	if !filepath.IsAbs(value.Home) {
		return value, fmt.Errorf("安装位置不是绝对路径")
	}
	return value, nil
}
func (m Manager) Install(ctx context.Context, root, source string, interval int) error {
	binaryData, e := os.ReadFile(source)
	if e != nil {
		return e
	}
	old, e := os.ReadFile(m.Binary())
	if e == nil {
		// 只替换本工具的旧 Python 启动器或与安装清单匹配的 Go 二进制。
		if !bytes.HasPrefix(old, []byte("#!/bin/sh\n# 由 gw 管理。\n")) {
			location, e := m.ReadLocation()
			if e != nil {
				return fmt.Errorf("命令路径已存在且无法确认归属：%w", e)
			}
			digest := sha256.Sum256(old)
			if hex.EncodeToString(digest[:]) != location.BinaryHash {
				return fmt.Errorf("现有 gw 命令与安装清单不匹配")
			}
		}
	} else if !os.IsNotExist(e) {
		return e
	}
	for _, dir := range []string{filepath.Dir(m.Binary()), filepath.Dir(m.Location()), filepath.Dir(m.Plist()), filepath.Join(root, "var")} {
		if e = os.MkdirAll(dir, 0700); e != nil {
			return e
		}
	}
	if e = state.Atomic(m.Binary(), binaryData, 0755); e != nil {
		return e
	}
	digest := sha256.Sum256(binaryData)
	if e = state.WriteJSON(m.Location(), Location{root, hex.EncodeToString(digest[:])}); e != nil {
		return e
	}
	payload := Plist(root, m.Binary(), interval)
	if e = state.Atomic(m.Plist(), payload, 0600); e != nil {
		return e
	}
	if data, e := exec.CommandContext(ctx, "/usr/bin/plutil", "-lint", m.Plist()).CombinedOutput(); e != nil {
		return fmt.Errorf("系统任务配置无效：%w\n%s", e, data)
	}
	registration, e := m.Registration(ctx)
	if e != nil {
		return e
	}
	if registration == Loaded {
		if _, e = command(ctx, "bootout", m.Target()); e != nil {
			return e
		}
	}
	_, e = command(ctx, "bootstrap", fmt.Sprintf("gui/%d", m.UID), m.Plist())
	return e
}
func (m Manager) Uninstall(ctx context.Context) error {
	if _, e := command(ctx, "bootout", m.Target()); e != nil {
		return e
	}
	return os.Remove(m.Plist())
}
