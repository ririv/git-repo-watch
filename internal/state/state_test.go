package state_test

import (
	"errors"
	"gw/internal/model"
	"gw/internal/state"
	"os"
	"path/filepath"
	"testing"
)

func TestConfigAtomicRoundtripAndPrivateMode(t *testing.T) {
	s := state.Store{Home: t.TempDir()}
	c := state.Config{Interval: 21600, Repositories: []string{}, Excluded: []string{}, PushPolicy: model.Disabled}
	if e := s.WriteConfig(c); e != nil {
		t.Fatal(e)
	}
	got, e := s.ReadConfig()
	if e != nil || got.Interval != 21600 {
		t.Fatal(got, e)
	}
	info, e := os.Stat(s.ConfigPath())
	if e != nil || info.Mode().Perm() != 0600 {
		t.Fatal(info, e)
	}
}
func TestLockActivityAndContention(t *testing.T) {
	s := state.Store{Home: t.TempDir()}
	a, e := s.Activity()
	if e != nil || a != state.Idle {
		t.Fatal(a, e)
	}
	lock, e := s.Lock()
	if e != nil {
		t.Fatal(e)
	}
	a, e = s.Activity()
	if e != nil || a != state.Running {
		t.Fatal(a, e)
	}
	if _, e = s.Lock(); !errors.Is(e, state.ErrRunning) {
		t.Fatal(e)
	}
	state.Unlock(lock)
	a, e = s.Activity()
	if e != nil || a != state.Idle {
		t.Fatal(a, e)
	}
}
func TestMissingAndMalformedReportsDiffer(t *testing.T) {
	s := state.Store{Home: t.TempDir()}
	if _, e := s.ReadReport(); !errors.Is(e, state.ErrNoReport) {
		t.Fatal(e)
	}
	os.Mkdir(s.StateDir(), 0700)
	os.WriteFile(filepath.Join(s.StateDir(), "latest.json"), []byte("not json"), 0600)
	if _, e := s.ReadReport(); e == nil || errors.Is(e, state.ErrNoReport) {
		t.Fatal(e)
	}
}
func TestPathContainment(t *testing.T) {
	root := filepath.Join(t.TempDir(), "scope")
	for _, item := range []struct {
		path string
		want bool
	}{{root, true}, {filepath.Join(root, "nested"), true}, {root + "-other", false}, {filepath.Dir(root), false}} {
		if got := state.Within(item.path, root); got != item.want {
			t.Fatalf("%s %v", item.path, got)
		}
	}
}
