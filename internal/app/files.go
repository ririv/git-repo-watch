package app

import (
	"context"
	"gw/internal/repoaction"
	"gw/internal/state"
)

func (s *Service) ReadFiles(ctx context.Context, path string) (repoaction.Files, error) {
	lock, e := s.repositoryLock(ctx, path)
	if e != nil {
		return nil, e
	}
	defer state.Unlock(lock)
	return repoaction.ReadFiles(ctx, s.Git, path)
}
func (s *Service) FileDiff(ctx context.Context, f repoaction.Files, c repoaction.Change) (string, error) {
	lock, e := s.repositoryLock(ctx, f.Path())
	if e != nil {
		return "", e
	}
	defer state.Unlock(lock)
	return repoaction.FileDiff(ctx, s.Git, f, c)
}
func (s *Service) EditIndex(ctx context.Context, f repoaction.Files, operation repoaction.IndexAction, selection []repoaction.Change) (repoaction.Files, error) {
	lock, e := s.repositoryLock(ctx, f.Path())
	if e != nil {
		return nil, e
	}
	defer state.Unlock(lock)
	return repoaction.EditIndex(ctx, s.Git, f, operation, selection)
}
