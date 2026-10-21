// Package app exposes UI-independent application operations and typed progress events.
package app

import (
	"context"
	"errors"
	"fmt"
	"gw/internal/check"
	"gw/internal/discovery"
	"gw/internal/gitcmd"
	"gw/internal/model"
	"gw/internal/notify"
	"gw/internal/scheduler"
	"gw/internal/state"
	"os"
	"path/filepath"
	"time"
)

type Event interface{ event() }
type Checking struct{ Path string }

func (Checking) event() {}

type Checked struct{ Repository model.Repository }

func (Checked) event() {}

type NotificationProblem struct{ Notice model.NoticeRecord }

func (NotificationProblem) event() {}

type Service struct {
	Store     state.Store
	Git       gitcmd.Runner
	Notifier  notify.Sender
	Scheduler scheduler.Reader
	Clock     func() time.Time
}

func New(home string, manager scheduler.Reader) *Service {
	return &Service{state.Store{Home: home}, gitcmd.OSRunner{Timeout: 120 * time.Second}, notify.MacOS{}, manager, time.Now}
}
func (s *Service) Config() (state.Config, error) { return s.Store.ReadConfig() }
func (s *Service) Report() (model.Report, error) { return s.Store.ReadReport() }
func (s *Service) Run(ctx context.Context, emit func(Event)) (model.Report, error) {
	lock, e := s.Store.Lock()
	if e != nil {
		return model.Report{}, e
	}
	defer state.Unlock(lock)
	config, e := s.Config()
	if e != nil {
		return model.Report{}, e
	}
	report := model.Report{SchemaVersion: 3, PushPolicy: config.PushPolicy, StartedAt: s.Clock().UTC().Format(time.RFC3339Nano), Repositories: []model.Repository{}, Notifications: []model.NoticeRecord{}}
	for _, path := range config.Repositories {
		if e = ctx.Err(); e != nil {
			return model.Report{}, e
		}
		emit(Checking{path})
		result := check.Repository(ctx, s.Git, path, config.PushPolicy).Record()
		if e = ctx.Err(); e != nil {
			return model.Report{}, e
		}
		for _, pushed := range result.Pushes {
			if pushed.Status == "failed" {
				notice := s.Notifier.Failure(ctx, path, pushed).Record()
				report.Notifications = append(report.Notifications, notice)
				if notice.Status == "failed" {
					emit(NotificationProblem{notice})
				}
			}
		}
		report.Repositories = append(report.Repositories, result)
		emit(Checked{result})
	}
	report.Status = model.Clean
	if len(report.Repositories) == 0 {
		report.Status = model.Unconfigured
	} else {
		for _, result := range report.Repositories {
			if result.Status != model.Clean {
				report.Status = model.Attention
				break
			}
		}
	}
	report.FinishedAt = s.Clock().UTC().Format(time.RFC3339Nano)
	if e = s.Store.WriteReport(report); e != nil {
		return report, e
	}
	return report, nil
}
func (s *Service) Register(paths []string) (int, error) {
	config, e := s.Config()
	if e != nil {
		return 0, e
	}
	seen := map[string]bool{}
	for _, p := range config.Repositories {
		seen[p] = true
	}
	added := 0
	for _, p := range paths {
		for _, x := range config.Excluded {
			if state.Within(p, x) {
				return 0, fmt.Errorf("仓库位于排除目录中：%s", p)
			}
		}
		if !seen[p] {
			seen[p] = true
			added++
			config.Repositories = append(config.Repositories, p)
		}
	}
	return added, s.Store.WriteConfig(config)
}
func (s *Service) Add(ctx context.Context, paths []string) (int, error) {
	roots := []string{}
	for _, path := range paths {
		path, e := filepath.Abs(path)
		if e != nil {
			return 0, e
		}
		root, e := gitcmd.Text(ctx, s.Git, path, "rev-parse", "--show-toplevel")
		if e != nil {
			return 0, e
		}
		root, e = filepath.EvalSymlinks(root)
		if e != nil {
			return 0, e
		}
		actual, e := filepath.EvalSymlinks(path)
		if e != nil {
			return 0, e
		}
		if root != actual {
			return 0, fmt.Errorf("路径不是仓库根目录：%s", path)
		}
		roots = append(roots, root)
	}
	return s.Register(roots)
}
func remove(paths []string, wanted string) ([]string, error) {
	out := []string{}
	found := false
	for _, p := range paths {
		if p == wanted {
			found = true
		} else {
			out = append(out, p)
		}
	}
	if !found {
		return nil, fmt.Errorf("列表中不存在：%s", wanted)
	}
	return out, nil
}
func canonical(path string) (string, error) {
	absolute, e := filepath.Abs(path)
	if e != nil {
		return "", e
	}
	_, e = os.Lstat(absolute)
	if e == nil {
		return filepath.EvalSymlinks(absolute)
	}
	if os.IsNotExist(e) {
		return filepath.Clean(absolute), nil
	}
	return "", e
}
func (s *Service) Remove(path string) error {
	path, e := canonical(path)
	if e != nil {
		return e
	}
	c, e := s.Config()
	if e != nil {
		return e
	}
	c.Repositories, e = remove(c.Repositories, path)
	if e != nil {
		return e
	}
	return s.Store.WriteConfig(c)
}
func (s *Service) Exclude(path string) error {
	path, e := canonical(path)
	if e != nil {
		return e
	}
	c, e := s.Config()
	if e != nil {
		return e
	}
	found := false
	for _, x := range c.Excluded {
		if path == x {
			found = true
		}
	}
	if !found {
		c.Excluded = append(c.Excluded, path)
	}
	kept := []string{}
	for _, repo := range c.Repositories {
		if !state.Within(repo, path) {
			kept = append(kept, repo)
		}
	}
	c.Repositories = kept
	return s.Store.WriteConfig(c)
}
func (s *Service) Unexclude(path string) error {
	path, e := canonical(path)
	if e != nil {
		return e
	}
	c, e := s.Config()
	if e != nil {
		return e
	}
	c.Excluded, e = remove(c.Excluded, path)
	if e != nil {
		return e
	}
	return s.Store.WriteConfig(c)
}
func (s *Service) Interval(seconds int) error {
	c, e := s.Config()
	if e != nil {
		return e
	}
	c.Interval = seconds
	return s.Store.WriteConfig(c)
}
func (s *Service) Policy(policy model.PushPolicy) error {
	c, e := s.Config()
	if e != nil {
		return e
	}
	c.PushPolicy = policy
	return s.Store.WriteConfig(c)
}

type ScanMode int

const (
	Preview ScanMode = iota
	Import
)

func (s *Service) Scan(ctx context.Context, root string, mode ScanMode) (discovery.Report, error) {
	c, e := s.Config()
	if e != nil {
		return discovery.Report{}, e
	}
	r, e := discovery.Discover(ctx, s.Git, root, c.Excluded)
	if e != nil {
		return r, e
	}
	r.ScannedAt = s.Clock().UTC().Format(time.RFC3339Nano)
	if e = os.MkdirAll(s.Store.StateDir(), 0700); e != nil {
		return r, e
	}
	if e = state.WriteJSON(filepath.Join(s.Store.StateDir(), "import-report.json"), r); e != nil {
		return r, e
	}
	if len(r.Repositories) == 0 {
		return r, fmt.Errorf("指定目录下未发现 Git 仓库")
	}
	if mode == Import {
		paths := []string{}
		for _, repo := range r.Repositories {
			paths = append(paths, repo.Path)
		}
		_, e = s.Register(paths)
	}
	return r, e
}

type Scope string

const (
	Current  Scope = "current"
	Outdated Scope = "outdated"
)

type ReportState interface{ reportState() }
type MissingReport struct{}

func (MissingReport) reportState() {}

type AvailableReport struct {
	Report model.Report
	Scope  Scope
}

func (AvailableReport) reportState()               {}
func (MissingReport) MarshalJSON() ([]byte, error) { return []byte(`{"status":"missing"}`), nil }

type Snapshot struct {
	Registration    scheduler.Registration `json:"registration"`
	Activity        state.Activity         `json:"activity"`
	Interval        int                    `json:"configured_interval_seconds"`
	Policy          model.PushPolicy       `json:"push_policy"`
	RepositoryCount int                    `json:"repository_count"`
	Excluded        []string               `json:"excluded_roots"`
	Schedule        scheduler.Schedule     `json:"schedule"`
	Report          ReportState            `json:"report"`
}

func (s *Service) Snapshot(ctx context.Context) (Snapshot, error) {
	c, e := s.Config()
	if e != nil {
		return Snapshot{}, e
	}
	registration, e := s.Scheduler.Registration(ctx)
	if e != nil {
		return Snapshot{}, e
	}
	schedule, e := s.Scheduler.Schedule()
	if e != nil {
		return Snapshot{}, e
	}
	activity, e := s.Store.Activity()
	if e != nil {
		return Snapshot{}, e
	}
	var report ReportState
	r, e := s.Report()
	if errors.Is(e, state.ErrNoReport) {
		report = MissingReport{}
	} else if e != nil {
		return Snapshot{}, e
	} else {
		scope := Current
		set := map[string]bool{}
		for _, path := range c.Repositories {
			set[path] = true
		}
		if len(set) != len(r.Repositories) {
			scope = Outdated
		} else {
			for _, repo := range r.Repositories {
				if !set[repo.Path] {
					scope = Outdated
					break
				}
			}
		}
		report = AvailableReport{r, scope}
	}
	return Snapshot{registration, activity, c.Interval, c.PushPolicy, len(c.Repositories), c.Excluded, schedule, report}, nil
}
