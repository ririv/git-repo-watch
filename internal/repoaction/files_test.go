package repoaction_test

import (
	"context"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"gw/internal/repoaction"
	"gw/internal/testkit"
)

func readFiles(t *testing.T, path string) repoaction.Files {
	t.Helper()
	f, e := repoaction.ReadFiles(context.Background(), testkit.Runner(), path)
	if e != nil {
		t.Fatal(e)
	}
	return f
}
func findFile(t *testing.T, f repoaction.Files, name string, area repoaction.Area) repoaction.Change {
	t.Helper()
	for _, c := range f.Changes() {
		if c.Name() == name && c.Area() == area {
			return c
		}
	}
	t.Fatalf("没有文件 %q 区域 %d: %v", name, area, f.Changes())
	return nil
}
func edit(t *testing.T, f repoaction.Files, op repoaction.IndexAction, items ...repoaction.Change) repoaction.Files {
	t.Helper()
	next, e := repoaction.EditIndex(context.Background(), testkit.Runner(), f, op, items)
	if e != nil {
		t.Fatal(e)
	}
	return next
}
func TestFilesSeparateIndexAndWorktreeDiff(t *testing.T) {
	f := testkit.New(t)
	path := filepath.Join(f.Repo, "file.txt")
	testkit.Write(t, path, "baseline\n")
	testkit.Git(t, f.Repo, "add", "file.txt")
	testkit.Git(t, f.Repo, "commit", "-m", "baseline")
	testkit.Write(t, path, "index version\n")
	testkit.Git(t, f.Repo, "add", "file.txt")
	testkit.Write(t, path, "working version\n")
	snapshot := readFiles(t, f.Repo)
	for area, want := range map[repoaction.Area]string{repoaction.Staged: "+index version", repoaction.Unstaged: "+working version"} {
		diff, e := repoaction.FileDiff(context.Background(), testkit.Runner(), snapshot, findFile(t, snapshot, "file.txt", area))
		if e != nil || !strings.Contains(diff, want) {
			t.Fatal(diff, e)
		}
	}
	next := edit(t, snapshot, repoaction.Unstage, findFile(t, snapshot, "file.txt", repoaction.Staged))
	findFile(t, next, "file.txt", repoaction.Unstaged)
	content, e := os.ReadFile(path)
	if e != nil || string(content) != "working version\n" {
		t.Fatal(string(content), e)
	}
}
func TestSelectedLiteralFilesAndIgnoredFiles(t *testing.T) {
	f := testkit.New(t)
	names := []string{"file[1].txt", "file1.txt", ":(glob)*", "-option", "换行\n文件"}
	for _, name := range names {
		testkit.Write(t, filepath.Join(f.Repo, name), name+"\n")
	}
	testkit.Write(t, filepath.Join(f.Repo, ".gitignore"), "secret\n")
	testkit.Write(t, filepath.Join(f.Repo, "secret"), "ignored")
	snapshot := readFiles(t, f.Repo)
	for _, c := range snapshot.Changes() {
		if c.Name() == "secret" {
			t.Fatal("不应列出忽略文件")
		}
	}
	for _, name := range []string{names[0], names[2], names[3], names[4]} {
		c := findFile(t, snapshot, name, repoaction.Untracked)
		diff, e := repoaction.FileDiff(context.Background(), testkit.Runner(), snapshot, c)
		if e != nil || !strings.Contains(diff, "new file") {
			t.Fatal(diff, e)
		}
		snapshot = edit(t, snapshot, repoaction.Stage, c)
	}
	findFile(t, snapshot, "file1.txt", repoaction.Untracked)
	for _, name := range []string{names[0], names[2], names[3], names[4]} {
		findFile(t, snapshot, name, repoaction.Staged)
	}
}
func TestRenameUnstageAndWorktreeModification(t *testing.T) {
	f := testkit.New(t)
	testkit.Write(t, filepath.Join(f.Repo, "file.txt"), strings.Repeat("baseline\n", 20))
	testkit.Git(t, f.Repo, "add", "file.txt")
	testkit.Git(t, f.Repo, "commit", "-m", "baseline")
	testkit.Git(t, f.Repo, "mv", "file.txt", "renamed.txt")
	testkit.Write(t, filepath.Join(f.Repo, "renamed.txt"), strings.Repeat("baseline\n", 20)+"changed after rename\n")
	snapshot := readFiles(t, f.Repo)
	snapshot = edit(t, snapshot, repoaction.Stage, findFile(t, snapshot, "renamed.txt", repoaction.Unstaged))
	snapshot = edit(t, snapshot, repoaction.Unstage, findFile(t, snapshot, "renamed.txt", repoaction.Staged))
	findFile(t, snapshot, "file.txt", repoaction.Unstaged)
	findFile(t, snapshot, "renamed.txt", repoaction.Untracked)
	if got := testkit.Git(t, f.Repo, "diff", "--cached"); got != "" {
		t.Fatal(got)
	}
}
func TestInitialIndexUnstagePreservesFile(t *testing.T) {
	testkit.Isolate(t)
	path := t.TempDir()
	testkit.Git(t, path, "init", "-b", "main")
	testkit.Write(t, filepath.Join(path, "first.txt"), "first\n")
	snapshot := readFiles(t, path)
	snapshot = edit(t, snapshot, repoaction.Stage, findFile(t, snapshot, "first.txt", repoaction.Untracked))
	snapshot = edit(t, snapshot, repoaction.Unstage, findFile(t, snapshot, "first.txt", repoaction.Staged))
	findFile(t, snapshot, "first.txt", repoaction.Untracked)
}
func TestIndexChangeRejectsStaleSelection(t *testing.T) {
	f := testkit.New(t)
	testkit.Write(t, filepath.Join(f.Repo, "new.txt"), "new")
	snapshot := readFiles(t, f.Repo)
	testkit.Write(t, filepath.Join(f.Repo, "file.txt"), "other")
	testkit.Git(t, f.Repo, "add", "file.txt")
	if _, e := repoaction.EditIndex(context.Background(), testkit.Runner(), snapshot, repoaction.Stage, []repoaction.Change{findFile(t, snapshot, "new.txt", repoaction.Untracked)}); e == nil {
		t.Fatal("旧快照不能操作新暂存区")
	}
}

func TestUntrackedSymlinkShowsTargetWithoutReadingIt(t *testing.T) {
	f := testkit.New(t)
	target := filepath.Join(f.Root, "private.txt")
	testkit.Write(t, target, "private target content")
	if e := os.Symlink(target, filepath.Join(f.Repo, "link")); e != nil {
		t.Fatal(e)
	}
	snapshot := readFiles(t, f.Repo)
	diff, e := repoaction.FileDiff(context.Background(), testkit.Runner(), snapshot, findFile(t, snapshot, "link", repoaction.Untracked))
	if e != nil || !strings.Contains(diff, "新符号链接") || strings.Contains(diff, "private target content") {
		t.Fatal(diff, e)
	}
}
