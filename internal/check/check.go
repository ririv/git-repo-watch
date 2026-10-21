// Package check inspects workspaces and tracked branches, optionally publishing ahead-only snapshots.
package check

import (
	"context"
	"fmt"
	"gw/internal/gitcmd"
	"gw/internal/model"
	"gw/internal/publish"
	"path/filepath"
	"strconv"
	"strings"
)

func Repository(ctx context.Context, g gitcmd.Runner, path string, policy model.PushPolicy) model.Result {
	pushes := []model.PushRecord{}
	issues := []model.Issue{}
	without := []model.BranchWithoutUpstream{}
	fail := func(e error) model.Result { return model.Failure(path, e.Error(), pushes) }
	text := func(args ...string) (string, error) { return gitcmd.Text(ctx, g, path, args...) }
	root, e := text("rev-parse", "--show-toplevel")
	if e != nil {
		return fail(e)
	}
	root, e = filepath.EvalSymlinks(root)
	if e != nil {
		return fail(e)
	}
	actual, e := filepath.EvalSymlinks(path)
	if e != nil {
		return fail(e)
	}
	if root != actual {
		return fail(fmt.Errorf("配置路径不是仓库根目录：%s", root))
	}
	if _, e = text("fetch", "--all", "--no-recurse-submodules"); e != nil {
		return fail(e)
	}
	working, e := text("status", "--porcelain=v1", "--untracked-files=all", "--ignore-submodules=none")
	if e != nil {
		return fail(e)
	}
	if working != "" {
		issues = append(issues, model.Issue{Kind: model.Dirty, Detail: "存在暂存、未暂存、未跟踪文件或子模块改动"})
	}
	headers, e := text("status", "--porcelain=v2", "--branch", "--untracked-files=no")
	if e != nil {
		return fail(e)
	}
	if strings.Contains("\n"+headers+"\n", "\n# branch.head (detached)\n") {
		issues = append(issues, model.Issue{Kind: model.Detached, Detail: "HEAD 处于游离状态"})
	}
	refs, e := text("for-each-ref", "--format=%(refname)%09%(upstream)", "refs/heads/")
	if e != nil {
		return fail(e)
	}
	if refs == "" {
		issues = append(issues, model.Issue{Kind: model.NoCommits, Detail: "没有可检查的本地分支提交"})
	}
	heads := map[string]map[string]string{}
	for _, row := range strings.Split(refs, "\n") {
		if row == "" {
			continue
		}
		if e = ctx.Err(); e != nil {
			return fail(e)
		}
		branch, upstream, ok := strings.Cut(row, "\t")
		if !ok {
			return fail(fmt.Errorf("Git 分支记录格式错误"))
		}
		name := strings.TrimPrefix(branch, "refs/heads/")
		if upstream == "" {
			without = append(without, model.BranchWithoutUpstream{Branch: name, Reason: model.NotConfigured})
			continue
		}
		remote, e := text("for-each-ref", "--format=%(upstream:remotename)", branch)
		if e != nil {
			return fail(e)
		}
		remoteRef, e := text("for-each-ref", "--format=%(upstream:remoteref)", branch)
		if e != nil {
			return fail(e)
		}
		if remote == "." {
			without = append(without, model.BranchWithoutUpstream{Branch: name, Reason: model.LocalUpstream})
			continue
		}
		if _, ok = heads[remote]; !ok {
			advertised, e := text("ls-remote", "--heads", remote)
			if e != nil {
				return fail(e)
			}
			heads[remote] = map[string]string{}
			for _, line := range strings.Split(advertised, "\n") {
				if line == "" {
					continue
				}
				oid, ref, ok := strings.Cut(line, "\t")
				if !ok {
					return fail(fmt.Errorf("远程分支记录格式错误"))
				}
				heads[remote][ref] = oid
			}
		}
		remoteOID, ok := heads[remote][remoteRef]
		if !ok {
			issues = append(issues, model.Issue{Kind: model.MissingUpstream, Detail: name + "：远程上游分支不存在"})
			continue
		}
		tracked, e := text("rev-parse", "--verify", upstream)
		if e != nil {
			return fail(e)
		}
		if tracked != remoteOID {
			return fail(fmt.Errorf("%s：远程在 fetch 后发生变化或 fetch refspec 未更新上游", name))
		}
		local, e := text("rev-parse", "--verify", branch)
		if e != nil {
			return fail(e)
		}
		counts, e := text("rev-list", "--left-right", "--count", local+"..."+tracked)
		if e != nil {
			return fail(e)
		}
		fields := strings.Fields(counts)
		if len(fields) != 2 {
			return fail(fmt.Errorf("提交数量格式错误"))
		}
		ahead, e := strconv.Atoi(fields[0])
		if e != nil {
			return fail(e)
		}
		behind, e := strconv.Atoi(fields[1])
		if e != nil {
			return fail(e)
		}
		if ahead > 0 && behind == 0 && policy == model.FastForward {
			target := model.PushTarget{Branch: name, LocalRef: branch, LocalOID: local, Remote: remote, RemoteRef: remoteRef, RemoteOID: remoteOID, Ahead: ahead}
			pushed := publish.Forward(ctx, g, path, target).Record()
			pushes = append(pushes, pushed)
			if pushed.Status == "pushed" {
				heads[remote][remoteRef] = local
				now, e := text("rev-parse", "--verify", branch)
				if e != nil {
					return fail(e)
				}
				if now != local {
					issues = append(issues, model.Issue{Kind: model.OutOfSync, Detail: name + "：推送后本地分支发生变化，待下次重新检查"})
				}
				continue
			}
			issues = append(issues, model.Issue{Kind: model.PushError, Detail: name + "：自动推送失败：" + pushed.Error})
		}
		if ahead > 0 || behind > 0 {
			issues = append(issues, model.Issue{Kind: model.OutOfSync, Detail: fmt.Sprintf("%s：领先远程 %d 个提交，落后远程 %d 个提交", name, ahead, behind)})
		}
	}
	if len(issues) > 0 {
		return model.NeedsAttention(path, issues[0], issues[1:], without, pushes)
	}
	return model.Normal(path, without, pushes)
}
