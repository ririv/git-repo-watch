package app

import (
	"context"
	"fmt"
	"os"
	"path/filepath"
	"time"

	"gw/internal/gitcmd"
	"gw/internal/model"
	"gw/internal/repoaction"
	"gw/internal/state"
)

func (s *Service) repositoryLock(ctx context.Context, path string) (*os.File, error) {
	if e := ctx.Err(); e != nil {
		return nil, e
	}
	lock, e := s.Store.Lock()
	if e != nil {
		return nil, e
	}
	fail := func(e error) (*os.File, error) { state.Unlock(lock); return nil, e }
	c, e := s.Config()
	if e != nil {
		return fail(e)
	}
	found := false
	for _, p := range c.Repositories {
		if p == path {
			found = true
			break
		}
	}
	if !found {
		return fail(fmt.Errorf("仓库不在当前注册列表中：%s", path))
	}
	root, e := gitcmd.Text(ctx, s.Git, path, "rev-parse", "--show-toplevel")
	if e != nil {
		return fail(e)
	}
	actual, e := filepath.EvalSymlinks(path)
	if e != nil {
		return fail(e)
	}
	root, e = filepath.EvalSymlinks(root)
	if e != nil {
		return fail(e)
	}
	if root != actual {
		return fail(fmt.Errorf("配置路径不是仓库根目录：%s", path))
	}
	return lock, nil
}
func (s *Service) PrepareCommit(ctx context.Context, path string) (repoaction.CommitPlan, error) {
	lock, e := s.repositoryLock(ctx, path)
	if e != nil {
		return nil, e
	}
	defer state.Unlock(lock)
	return repoaction.PrepareCommit(ctx, s.Git, path)
}
func (s *Service) CommitRepository(ctx context.Context, p repoaction.CommitPlan, message string) (repoaction.CommitResult, error) {
	lock, e := s.repositoryLock(ctx, p.Path())
	if e != nil {
		return repoaction.CommitResult{}, e
	}
	defer state.Unlock(lock)
	result, e := repoaction.Commit(ctx, s.Git, p, message)
	if e != nil {
		return result, e
	}
	record := struct {
		At     string                  `json:"at"`
		Path   string                  `json:"path"`
		Branch string                  `json:"branch"`
		Result repoaction.CommitResult `json:"result"`
	}{s.Clock().UTC().Format(time.RFC3339Nano), p.Path(), p.Branch(), result}
	if e = state.WriteJSON(filepath.Join(s.Store.StateDir(), "manual-commit.json"), record); e != nil {
		return result, fmt.Errorf("提交 %s 已生成，但保存操作记录失败：%w", result.OID, e)
	}
	return result, nil
}
func (s *Service) PreparePush(ctx context.Context, path string) (repoaction.PushPlan, error) {
	lock, e := s.repositoryLock(ctx, path)
	if e != nil {
		return nil, e
	}
	defer state.Unlock(lock)
	return repoaction.PreparePush(ctx, s.Git, path)
}

func (s *Service) PushRepository(ctx context.Context, p repoaction.PushPlan) (model.Push, error) {
	lock, e := s.repositoryLock(ctx, p.Path())
	if e != nil {
		return nil, e
	}
	defer state.Unlock(lock)
	result := repoaction.Push(ctx, s.Git, p)
	record := struct {
		At   string           `json:"at"`
		Path string           `json:"path"`
		Push model.PushRecord `json:"push"`
	}{s.Clock().UTC().Format(time.RFC3339Nano), p.Path(), result.Record()}
	if e = state.WriteJSON(filepath.Join(s.Store.StateDir(), "manual-push.json"), record); e != nil {
		return result, fmt.Errorf("推送结果为 %s，但保存操作记录失败：%w", result.Record().Status, e)
	}
	return result, nil
}
