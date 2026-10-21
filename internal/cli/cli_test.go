package cli_test

import (
	"bytes"
	"context"
	"encoding/json"
	"gw/internal/app"
	"gw/internal/cli"
	"gw/internal/model"
	"gw/internal/scheduler"
	"gw/internal/state"
	"strings"
	"testing"
	"time"
)

type forbiddenGit struct{ t *testing.T }

func (g forbiddenGit) Run(context.Context, string, ...string) ([]byte, error) {
	g.t.Fatal("管理命令不应调用 Git")
	return nil, nil
}

type reader struct{}

func (reader) Registration(context.Context) (scheduler.Registration, error) {
	return scheduler.Loaded, nil
}
func (reader) Schedule() (scheduler.Schedule, error) { return scheduler.Saved{Interval: 21600}, nil }

type installer struct{ calls int }

func (i *installer) Install(context.Context, string, string, int) error { i.calls++; return nil }
func (i *installer) Uninstall(context.Context) error                    { i.calls++; return nil }
func service(t *testing.T) *app.Service {
	t.Helper()
	s := app.New(t.TempDir(), reader{})
	s.Git = forbiddenGit{t}
	c := state.Config{Interval: 21600, Repositories: []string{}, Excluded: []string{}, PushPolicy: model.Disabled}
	if e := s.Store.WriteConfig(c); e != nil {
		t.Fatal(e)
	}
	return s
}
func invoke(s *app.Service, install *installer, args ...string) (int, string, string) {
	var out, err bytes.Buffer
	code := cli.Run(context.Background(), s, install, "/path/to/gw", args, &out, &err)
	return code, out.String(), err.String()
}
func writeReport(t *testing.T, s *app.Service) {
	t.Helper()
	lock, e := s.Store.Lock()
	if e != nil {
		t.Fatal(e)
	}
	state.Unlock(lock)
	r := model.Report{SchemaVersion: 3, PushPolicy: model.Disabled, StartedAt: time.Now().UTC().Format(time.RFC3339Nano), FinishedAt: time.Now().UTC().Format(time.RFC3339Nano), Status: model.Attention, Notifications: []model.NoticeRecord{}, Repositories: []model.Repository{
		model.Normal("/path/to/clean", []model.BranchWithoutUpstream{{Branch: "experiment", Reason: model.NotConfigured}}, nil).Record(),
		model.NeedsAttention("/path/to/dirty", model.Issue{Kind: model.Dirty, Detail: "有未提交改动"}, nil, []model.BranchWithoutUpstream{{Branch: "feature", Reason: model.NotConfigured}}, nil).Record(),
	}}
	if e = s.Store.WriteReport(r); e != nil {
		t.Fatal(e)
	}
}
func TestDefaultStatusNeverChecks(t *testing.T) {
	s := service(t)
	i := &installer{}
	code, out, err := invoke(s, i)
	if code != 0 || !strings.Contains(out, "尚未生成完整报告") || err != "" || i.calls != 0 {
		t.Fatal(code, out, err)
	}
}
func TestJSONStatusAndReportFilters(t *testing.T) {
	s := service(t)
	writeReport(t, s)
	i := &installer{}
	code, out, err := invoke(s, i, "status", "--json")
	if code != 0 {
		t.Fatal(err)
	}
	var snapshot map[string]any
	if e := json.Unmarshal([]byte(out), &snapshot); e != nil {
		t.Fatal(e)
	}
	if snapshot["activity"] != "idle" {
		t.Fatal(snapshot)
	}
	for _, item := range []struct {
		filter string
		want   int
	}{{"--problems", 1}, {"--no-upstream", 2}, {"--pushes", 0}} {
		code, out, err := invoke(s, i, "report", "--json", item.filter)
		if code != 0 {
			t.Fatal(err)
		}
		var r model.Report
		if e := json.Unmarshal([]byte(out), &r); e != nil {
			t.Fatal(e)
		}
		if len(r.Repositories) != item.want {
			t.Fatal(out)
		}
	}
}
func TestMissingReportDoesNotCheck(t *testing.T) {
	s := service(t)
	code, out, err := invoke(s, &installer{}, "report")
	if code != 1 || !strings.Contains(out, "尚未生成") || err != "" {
		t.Fatal(code, out, err)
	}
}
func TestConfigMutationsDoNotApplyOrCheck(t *testing.T) {
	s := service(t)
	i := &installer{}
	for _, args := range [][]string{{"config"}, {"interval", "8h"}, {"autopush", "fast_forward"}, {"exclude", "/path/to/excluded"}, {"unexclude", "/path/to/excluded"}} {
		code, _, err := invoke(s, i, args...)
		if code != 0 {
			t.Fatal(args, err)
		}
	}
	c, e := s.Config()
	if e != nil || c.Interval != 28800 || c.PushPolicy != model.FastForward || len(c.Excluded) != 0 || i.calls != 0 {
		t.Fatal(c, e, i.calls)
	}
	code, out, err := invoke(s, i, "status")
	if code != 0 || !strings.Contains(out, "频率尚未应用") {
		t.Fatal(out, err)
	}
}
func TestInstallIsExplicitAndDoesNotCheck(t *testing.T) {
	s := service(t)
	i := &installer{}
	code, _, err := invoke(s, i, "install")
	if code != 0 || i.calls != 1 {
		t.Fatal(code, err, i.calls)
	}
}
func TestMalformedReportIsAnError(t *testing.T) {
	s := service(t)
	lock, e := s.Store.Lock()
	if e != nil {
		t.Fatal(e)
	}
	state.Unlock(lock)
	if e = state.Atomic(s.Store.StateDir()+"/latest.json", []byte("not json"), 0600); e != nil {
		t.Fatal(e)
	}
	code, _, err := invoke(s, &installer{}, "report")
	if code != 2 || !strings.Contains(err, "错误") {
		t.Fatal(code, err)
	}
}
func TestDurationUnits(t *testing.T) {
	for _, item := range []struct {
		in      string
		seconds int
	}{{"6h", 21600}, {"30m", 1800}, {"1d", 86400}, {"60s", 60}, {"21600", 21600}} {
		n, e := cli.ParseInterval(item.in)
		if e != nil || n != item.seconds {
			t.Fatal(item, n, e)
		}
	}
}
func TestHelpWithoutInstallation(t *testing.T) {
	var out, err bytes.Buffer
	code := cli.Execute(context.Background(), []string{"--help"}, &out, &err)
	if code != 0 || !strings.Contains(out.String(), "autopush") {
		t.Fatal(code, out.String(), err.String())
	}
}
