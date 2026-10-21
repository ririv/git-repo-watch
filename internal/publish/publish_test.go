package publish_test

import (
	"context"
	"gw/internal/gitcmd"
	"gw/internal/model"
	"gw/internal/publish"
	"gw/internal/testkit"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestSuccessAndExactRefspec(t *testing.T) {
	f := testkit.New(t)
	f.Commit(t, "ahead")
	testkit.Git(t, f.Repo, "branch", "other")
	testkit.Git(t, f.Repo, "tag", "-a", "local-tag", "-m", "tag")
	testkit.Git(t, f.Repo, "config", "remote.origin.mirror", "true")
	testkit.Git(t, f.Repo, "config", "push.followTags", "true")
	testkit.Git(t, f.Repo, "config", "remote.origin.push", "refs/heads/other:refs/heads/other")
	r := publish.Forward(context.Background(), testkit.Runner(), f.Repo, f.Target(t)).Record()
	if r.Status != "pushed" {
		t.Fatal(r)
	}
	if got := testkit.Git(t, f.Remote, "for-each-ref", "--format=%(refname)"); got != "refs/heads/main" {
		t.Fatal(got)
	}
}
func TestServerRejectionPreservesRemote(t *testing.T) {
	f := testkit.New(t)
	f.Commit(t, "ahead")
	target := f.Target(t)
	hook := filepath.Join(f.Remote, "hooks/pre-receive")
	testkit.Write(t, hook, "#!/bin/sh\necho rejected >&2\nexit 1\n")
	os.Chmod(hook, 0700)
	r := publish.Forward(context.Background(), testkit.Runner(), f.Repo, target).Record()
	if r.Status != "failed" || r.Phase != model.Transfer || !strings.Contains(r.Error, "rejected") {
		t.Fatal(r)
	}
	if got := testkit.Git(t, f.Remote, "rev-parse", "main"); got != target.RemoteOID {
		t.Fatal(got)
	}
}
func TestChangedLocalOrRemoteBlocked(t *testing.T) {
	for _, which := range []string{"local", "remote"} {
		t.Run(which, func(t *testing.T) {
			f := testkit.New(t)
			f.Commit(t, "ahead")
			target := f.Target(t)
			if which == "local" {
				f.Commit(t, "new")
			} else {
				f.Advance(t)
			}
			before := testkit.Git(t, f.Remote, "rev-parse", "main")
			r := publish.Forward(context.Background(), testkit.Runner(), f.Repo, target).Record()
			if r.Status != "failed" || r.Phase != model.Validate {
				t.Fatal(r)
			}
			if testkit.Git(t, f.Remote, "rev-parse", "main") != before {
				t.Fatal("改写了远程")
			}
		})
	}
}

type racing struct {
	gitcmd.Runner
	before func()
}

func (r racing) Run(ctx context.Context, dir string, args ...string) ([]byte, error) {
	for _, arg := range args {
		if arg == "push" {
			r.before()
			break
		}
	}
	return r.Runner.Run(ctx, dir, args...)
}
func TestServerRaceUsesNormalRejection(t *testing.T) {
	f := testkit.New(t)
	f.Commit(t, "ahead")
	target := f.Target(t)
	newTip := ""
	g := racing{testkit.Runner(), func() { f.Advance(t); newTip = testkit.Git(t, f.Remote, "rev-parse", "main") }}
	r := publish.Forward(context.Background(), g, f.Repo, target).Record()
	if r.Status != "failed" || r.Phase != model.Transfer {
		t.Fatal(r)
	}
	if testkit.Git(t, f.Remote, "rev-parse", "main") != newTip {
		t.Fatal("覆盖了并发提交")
	}
}
func TestAlternatePushURLBlocked(t *testing.T) {
	f := testkit.New(t)
	f.Commit(t, "ahead")
	testkit.Git(t, f.Repo, "config", "remote.origin.pushurl", filepath.Join(f.Root, "other.git"))
	r := publish.Forward(context.Background(), testkit.Runner(), f.Repo, f.Target(t)).Record()
	if r.Status != "failed" || r.Phase != model.Validate {
		t.Fatal(r)
	}
}
func TestUnfinishedMergeBlocked(t *testing.T) {
	f := testkit.New(t)
	testkit.Git(t, f.Repo, "checkout", "-b", "side")
	f.Commit(t, "side")
	testkit.Git(t, f.Repo, "checkout", "main")
	f.Commit(t, "main")
	target := f.Target(t)
	testkit.Git(t, f.Repo, "merge", "--no-commit", "--no-ff", "side")
	r := publish.Forward(context.Background(), testkit.Runner(), f.Repo, target).Record()
	if r.Status != "failed" || !strings.Contains(r.Error, "未完成") {
		t.Fatal(r)
	}
}
