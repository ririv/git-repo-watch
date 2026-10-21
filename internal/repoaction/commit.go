// Package repoaction prepares immutable reviews for explicit single-repository mutations.
package repoaction

import (
	"context"
	"fmt"
	"strings"

	"gw/internal/gitcmd"
)

type CommitPlan interface {
	commitPlan()
	Path() string
	Branch() string
	Files() []string
	Stat() string
}
type commitPlan struct {
	path  string
	head  gitcmd.Head
	tree  string
	files []string
	stat  string
}

func (*commitPlan) commitPlan()       {}
func (p *commitPlan) Path() string    { return p.path }
func (p *commitPlan) Branch() string  { return p.head.Branch() }
func (p *commitPlan) Files() []string { return append([]string{}, p.files...) }
func (p *commitPlan) Stat() string    { return p.stat }

func PrepareCommit(ctx context.Context, g gitcmd.Runner, path string) (CommitPlan, error) {
	if e := gitcmd.RequireIdle(ctx, g, path); e != nil {
		return nil, e
	}
	head, e := gitcmd.ReadHead(ctx, g, path)
	if e != nil {
		return nil, e
	}
	tree, e := gitcmd.Text(ctx, g, path, "write-tree")
	if e != nil {
		return nil, e
	}
	data, e := g.Run(ctx, path, "diff", "--cached", "--name-only", "-z", "--no-ext-diff", "--no-textconv")
	if e != nil {
		return nil, e
	}
	files := []string{}
	for _, file := range strings.Split(string(data), "\x00") {
		if file != "" {
			files = append(files, file)
		}
	}
	if len(files) == 0 {
		return nil, fmt.Errorf("没有已暂存的改动；请先使用 git add 选择要提交的文件")
	}
	stat, e := gitcmd.Text(ctx, g, path, "diff", "--cached", "--stat", "--no-color", "--no-ext-diff", "--no-textconv")
	if e != nil {
		return nil, e
	}
	p := &commitPlan{path, head, tree, files, stat}
	if e = validateCommit(ctx, g, p); e != nil {
		return nil, e
	}
	return p, nil
}

func validateCommit(ctx context.Context, g gitcmd.Runner, p *commitPlan) error {
	if e := gitcmd.RequireIdle(ctx, g, p.path); e != nil {
		return e
	}
	current, e := gitcmd.ReadHead(ctx, g, p.path)
	if e != nil {
		return e
	}
	if current != p.head {
		return fmt.Errorf("分支或 HEAD 已变化，请重新查看提交内容")
	}
	tree, e := gitcmd.Text(ctx, g, p.path, "write-tree")
	if e != nil {
		return e
	}
	if tree != p.tree {
		return fmt.Errorf("暂存内容已变化，请重新查看提交内容")
	}
	return nil
}

type CommitReview string

const (
	ExactCommit   CommitReview = "exact"
	ChangedCommit CommitReview = "changed"
)

type CommitResult struct {
	OID    string       `json:"oid"`
	Review CommitReview `json:"review"`
}

func Commit(ctx context.Context, g gitcmd.Runner, review CommitPlan, message string) (CommitResult, error) {
	p := review.(*commitPlan)
	if strings.TrimSpace(message) == "" || strings.ContainsRune(message, '\x00') {
		return CommitResult{}, fmt.Errorf("提交说明不能为空且不能包含 NUL")
	}
	if e := validateCommit(ctx, g, p); e != nil {
		return CommitResult{}, e
	}
	if _, e := g.Run(ctx, p.path, "commit", "-m", message); e != nil {
		return CommitResult{}, e
	}
	oid, e := gitcmd.Text(ctx, g, p.path, "rev-parse", "--verify", "HEAD")
	if e != nil {
		return CommitResult{}, fmt.Errorf("提交命令已完成，但无法核验提交，请先查看 Git 日志：%w", e)
	}
	tree, e := gitcmd.Text(ctx, g, p.path, "rev-parse", "--verify", oid+"^{tree}")
	if e != nil {
		return CommitResult{}, fmt.Errorf("提交 %s 已生成，但无法核验内容：%w", oid, e)
	}
	result := CommitResult{oid, ExactCommit}
	if tree != p.tree {
		result.Review = ChangedCommit
	}
	return result, nil
}
