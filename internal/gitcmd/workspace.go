package gitcmd

import (
	"context"
	"fmt"
	"os"
	"strings"
)

// RequireIdle rejects unfinished operations and unresolved index entries.
func RequireIdle(ctx context.Context, g Runner, path string) error {
	for _, operation := range []string{"MERGE_HEAD", "CHERRY_PICK_HEAD", "REVERT_HEAD", "rebase-merge", "rebase-apply", "sequencer"} {
		location, e := Text(ctx, g, path, "rev-parse", "--path-format=absolute", "--git-path", operation)
		if e != nil {
			return e
		}
		_, e = os.Stat(location)
		if e == nil {
			return fmt.Errorf("仓库存在未完成的 Git 操作：%s", operation)
		}
		if !os.IsNotExist(e) {
			return e
		}
	}
	conflicts, e := Text(ctx, g, path, "ls-files", "--unmerged", "-z")
	if e != nil {
		return e
	}
	if conflicts != "" {
		return fmt.Errorf("仓库存在未解决的冲突")
	}
	return nil
}

// AttachedHead returns a symbolic local branch and either an initial or existing commit base.
type Head interface {
	head()
	Branch() string
}
type InitialHead struct{ Ref string }
type ExistingHead struct{ Ref, OID string }

func (InitialHead) head()             {}
func (ExistingHead) head()            {}
func (h InitialHead) Branch() string  { return h.Ref }
func (h ExistingHead) Branch() string { return h.Ref }
func ReadHead(ctx context.Context, g Runner, path string) (Head, error) {
	output, e := Text(ctx, g, path, "status", "--porcelain=v2", "--branch", "--untracked-files=no")
	if e != nil {
		return nil, e
	}
	branch, oid := "", ""
	for _, line := range strings.Split(output, "\n") {
		if strings.HasPrefix(line, "# branch.head ") {
			branch = strings.TrimPrefix(line, "# branch.head ")
		}
		if strings.HasPrefix(line, "# branch.oid ") {
			oid = strings.TrimPrefix(line, "# branch.oid ")
		}
	}
	if branch == "(detached)" {
		return nil, fmt.Errorf("HEAD 处于游离状态，请先切换到本地分支")
	}
	if branch == "" || oid == "" {
		return nil, fmt.Errorf("Git 未返回完整的分支状态")
	}
	if oid == "(initial)" {
		return InitialHead{branch}, nil
	}
	return ExistingHead{branch, oid}, nil
}
