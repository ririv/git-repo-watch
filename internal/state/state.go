// Package state stores private configuration, reports, and process locks.
package state

import (
	"encoding/json"
	"errors"
	"fmt"
	"gw/internal/model"
	"os"
	"path/filepath"
	"strings"
	"syscall"
)

type Config struct {
	Interval     int              `json:"interval_seconds"`
	Repositories []string         `json:"repositories"`
	Excluded     []string         `json:"excluded_roots"`
	PushPolicy   model.PushPolicy `json:"push_policy"`
}

func Within(path, root string) bool {
	relative, err := filepath.Rel(root, path)
	return err == nil && relative != ".." && !filepath.IsAbs(relative) && !strings.HasPrefix(relative, ".."+string(os.PathSeparator))
}
func (c Config) Validate() error {
	if c.Interval < 60 {
		return fmt.Errorf("检查间隔至少为 60 秒")
	}
	if _, e := model.ParsePolicy(string(c.PushPolicy)); e != nil {
		return e
	}
	for _, paths := range [][]string{c.Repositories, c.Excluded} {
		seen := map[string]struct{}{}
		for _, path := range paths {
			if !filepath.IsAbs(path) || filepath.Clean(path) != path {
				return fmt.Errorf("路径必须是规范绝对路径：%s", path)
			}
			if _, ok := seen[path]; ok {
				return fmt.Errorf("重复路径：%s", path)
			}
			seen[path] = struct{}{}
		}
	}
	for _, repo := range c.Repositories {
		for _, excluded := range c.Excluded {
			if Within(repo, excluded) {
				return fmt.Errorf("仓库处于已排除目录：%s", repo)
			}
		}
	}
	return nil
}

type Store struct{ Home string }

func (s Store) ConfigPath() string { return filepath.Join(s.Home, "config.json") }
func (s Store) StateDir() string   { return filepath.Join(s.Home, "var") }
func (s Store) ReadConfig() (Config, error) {
	var c Config
	data, e := os.ReadFile(s.ConfigPath())
	if e != nil {
		return c, e
	}
	if e = json.Unmarshal(data, &c); e != nil {
		return c, e
	}
	return c, c.Validate()
}
func (s Store) WriteConfig(c Config) error {
	if e := c.Validate(); e != nil {
		return e
	}
	return WriteJSON(s.ConfigPath(), c)
}
func Atomic(path string, data []byte, mode os.FileMode) error {
	file, e := os.CreateTemp(filepath.Dir(path), ".gw-*")
	if e != nil {
		return e
	}
	name := file.Name()
	defer os.Remove(name)
	if e = file.Chmod(mode); e != nil {
		file.Close()
		return e
	}
	if _, e = file.Write(data); e != nil {
		file.Close()
		return e
	}
	if e = file.Sync(); e != nil {
		file.Close()
		return e
	}
	if e = file.Close(); e != nil {
		return e
	}
	return os.Rename(name, path)
}
func WriteJSON(path string, v any) error {
	data, e := json.MarshalIndent(v, "", "  ")
	if e != nil {
		return e
	}
	return Atomic(path, append(data, '\n'), 0600)
}
func (s Store) WriteReport(r model.Report) error {
	return WriteJSON(filepath.Join(s.StateDir(), "latest.json"), r)
}

var ErrNoReport = errors.New("尚未生成完整报告")

func (s Store) ReadReport() (model.Report, error) {
	var r model.Report
	data, e := os.ReadFile(filepath.Join(s.StateDir(), "latest.json"))
	if os.IsNotExist(e) {
		return r, ErrNoReport
	}
	if e != nil {
		return r, e
	}
	if e = json.Unmarshal(data, &r); e != nil {
		return r, e
	}
	return r, r.Validate()
}

var ErrRunning = errors.New("已有检查正在运行")

func (s Store) Lock() (*os.File, error) {
	if e := os.MkdirAll(s.StateDir(), 0700); e != nil {
		return nil, e
	}
	f, e := os.OpenFile(filepath.Join(s.StateDir(), "check.lock"), os.O_CREATE|os.O_RDWR, 0600)
	if e != nil {
		return nil, e
	}
	if e = syscall.Flock(int(f.Fd()), syscall.LOCK_EX|syscall.LOCK_NB); e != nil {
		f.Close()
		if e == syscall.EWOULDBLOCK {
			return nil, ErrRunning
		}
		return nil, e
	}
	return f, nil
}
func Unlock(f *os.File) { syscall.Flock(int(f.Fd()), syscall.LOCK_UN); f.Close() }

type Activity string

const (
	Idle    Activity = "idle"
	Running Activity = "running"
)

func (s Store) Activity() (Activity, error) {
	f, e := os.Open(filepath.Join(s.StateDir(), "check.lock"))
	if os.IsNotExist(e) {
		return Idle, nil
	}
	if e != nil {
		return "", e
	}
	defer f.Close()
	e = syscall.Flock(int(f.Fd()), syscall.LOCK_EX|syscall.LOCK_NB)
	if e == syscall.EWOULDBLOCK {
		return Running, nil
	}
	if e != nil {
		return "", e
	}
	syscall.Flock(int(f.Fd()), syscall.LOCK_UN)
	return Idle, nil
}
