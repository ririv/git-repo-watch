// Package testkit provides isolated local Git fixtures without user configuration or network access.
package testkit

import (
	"context"
	"fmt"
	"gw/internal/gitcmd"
	"gw/internal/model"
	"os"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"
	"testing"
	"time"
)

type Fixture struct {
	Root, Repo, Remote string
	counter            int
}

func Isolate(t *testing.T) {
	t.Helper()
	t.Setenv("GIT_CONFIG_NOSYSTEM", "1")
	t.Setenv("GIT_CONFIG_GLOBAL", "/dev/null")
	t.Setenv("GIT_TERMINAL_PROMPT", "0")
}
func Git(t *testing.T, dir string, args ...string) string {
	t.Helper()
	all := append([]string{"-C", dir}, args...)
	cmd := exec.Command("/usr/bin/git", all...)
	data, e := cmd.CombinedOutput()
	if e != nil {
		t.Fatalf("git %v：%v\n%s", args, e, data)
	}
	return strings.TrimSuffix(string(data), "\n")
}
func Init(t *testing.T, path string) {
	t.Helper()
	if e := os.MkdirAll(path, 0700); e != nil {
		t.Fatal(e)
	}
	Git(t, path, "init", "--initial-branch=main")
	Git(t, path, "config", "user.name", "测试")
	Git(t, path, "config", "user.email", "test@example.invalid")
}
func New(t *testing.T) *Fixture {
	t.Helper()
	Isolate(t)
	root := t.TempDir()
	root, e := filepath.EvalSymlinks(root)
	if e != nil {
		t.Fatal(e)
	}
	f := &Fixture{Root: root, Repo: filepath.Join(root, "repo"), Remote: filepath.Join(root, "remote.git")}
	Init(t, f.Repo)
	if e = os.Mkdir(f.Remote, 0700); e != nil {
		t.Fatal(e)
	}
	Git(t, f.Remote, "init", "--bare", "--initial-branch=main")
	Git(t, f.Repo, "commit", "--allow-empty", "-m", "initial")
	Git(t, f.Repo, "remote", "add", "origin", f.Remote)
	Git(t, f.Repo, "push", "-u", "origin", "main")
	return f
}
func (f *Fixture) Commit(t *testing.T, message string) {
	t.Helper()
	f.counter++
	Git(t, f.Repo, "commit", "--allow-empty", "-m", fmt.Sprintf("%s-%d", message, f.counter))
}
func (f *Fixture) Advance(t *testing.T) {
	t.Helper()
	peer := filepath.Join(t.TempDir(), "peer")
	Git(t, f.Root, "clone", f.Remote, peer)
	Git(t, peer, "config", "user.name", "测试")
	Git(t, peer, "config", "user.email", "test@example.invalid")
	Git(t, peer, "commit", "--allow-empty", "-m", "remote advance")
	Git(t, peer, "push")
}
func (f *Fixture) Target(t *testing.T) model.PushTarget {
	t.Helper()
	local := Git(t, f.Repo, "rev-parse", "main")
	remote := Git(t, f.Remote, "rev-parse", "main")
	ahead, e := strconv.Atoi(Git(t, f.Repo, "rev-list", "--count", remote+".."+local))
	if e != nil {
		t.Fatal(e)
	}
	return model.PushTarget{Branch: "main", LocalRef: "refs/heads/main", LocalOID: local, Remote: "origin", RemoteRef: "refs/heads/main", RemoteOID: remote, Ahead: ahead}
}
func Runner() gitcmd.Runner { return gitcmd.OSRunner{Timeout: 10 * time.Second} }
func Run(t *testing.T, g gitcmd.Runner, dir string, args ...string) string {
	t.Helper()
	text, e := gitcmd.Text(context.Background(), g, dir, args...)
	if e != nil {
		t.Fatal(e)
	}
	return text
}
func Write(t *testing.T, path, text string) {
	t.Helper()
	if e := os.MkdirAll(filepath.Dir(path), 0700); e != nil {
		t.Fatal(e)
	}
	if e := os.WriteFile(path, []byte(text), 0600); e != nil {
		t.Fatal(e)
	}
}
