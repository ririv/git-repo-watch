// Package publish performs narrowly scoped, non-forced publication of commit snapshots.
package publish

import (
	"context"
	"fmt"
	"gw/internal/gitcmd"
	"gw/internal/model"
	"os"
	"strings"
)

func RemoteHead(ctx context.Context, g gitcmd.Runner, path, remote, ref string) (string, error) {
	output, e := gitcmd.Text(ctx, g, path, "ls-remote", "--heads", "--", remote, ref)
	if e != nil {
		return "", e
	}
	matches := []string{}
	for _, line := range strings.Split(output, "\n") {
		oid, name, ok := strings.Cut(line, "\t")
		if ok && name == ref {
			matches = append(matches, oid)
		}
	}
	if len(matches) != 1 {
		return "", fmt.Errorf("无法唯一确认远程分支：%s/%s", remote, ref)
	}
	return matches[0], nil
}
func Forward(ctx context.Context, g gitcmd.Runner, path string, target model.PushTarget) model.Push {
	phase := model.Validate
	fail := func(e error) model.Push { return model.PublicationFailed(target, phase, e.Error()) }
	if !strings.HasPrefix(target.LocalRef, "refs/heads/") || !strings.HasPrefix(target.RemoteRef, "refs/heads/") || target.Ahead <= 0 {
		return fail(fmt.Errorf("无效的自动推送目标"))
	}
	for _, operation := range []string{"MERGE_HEAD", "CHERRY_PICK_HEAD", "REVERT_HEAD", "rebase-merge", "rebase-apply", "sequencer"} {
		location, e := gitcmd.Text(ctx, g, path, "rev-parse", "--path-format=absolute", "--git-path", operation)
		if e != nil {
			return fail(e)
		}
		_, e = os.Stat(location)
		if e == nil {
			return fail(fmt.Errorf("仓库存在未完成的 Git 操作：%s，取消自动推送", operation))
		}
		if !os.IsNotExist(e) {
			return fail(e)
		}
	}
	headers, e := gitcmd.Text(ctx, g, path, "status", "--porcelain=v2", "--branch", "--untracked-files=no")
	if e != nil {
		return fail(e)
	}
	if strings.Contains("\n"+headers+"\n", "\n# branch.head (detached)\n") {
		return fail(fmt.Errorf("HEAD 处于游离状态，取消自动推送"))
	}
	conflicts, e := gitcmd.Text(ctx, g, path, "ls-files", "--unmerged", "-z")
	if e != nil {
		return fail(e)
	}
	if conflicts != "" {
		return fail(fmt.Errorf("仓库存在未解决的冲突，取消自动推送"))
	}
	fetch, e := gitcmd.Text(ctx, g, path, "remote", "get-url", "--all", target.Remote)
	if e != nil {
		return fail(e)
	}
	push, e := gitcmd.Text(ctx, g, path, "remote", "get-url", "--push", "--all", target.Remote)
	if e != nil {
		return fail(e)
	}
	if strings.Contains(push, "\n") || push != fetch {
		return fail(fmt.Errorf("远程推送地址与检查地址不同或存在多个推送地址"))
	}
	local, e := gitcmd.Text(ctx, g, path, "rev-parse", "--verify", target.LocalRef)
	if e != nil {
		return fail(e)
	}
	if local != target.LocalOID {
		return fail(fmt.Errorf("本地分支在检查后发生变化，取消自动推送"))
	}
	remote, e := RemoteHead(ctx, g, path, target.Remote, target.RemoteRef)
	if e != nil {
		return fail(e)
	}
	if remote != target.RemoteOID {
		return fail(fmt.Errorf("远程分支在检查后发生变化，取消自动推送"))
	}
	if _, e = g.Run(ctx, path, "merge-base", "--is-ancestor", target.RemoteOID, target.LocalOID); e != nil {
		return fail(e)
	}
	phase = model.Transfer
	// 提交 ID 固定本次发布范围；显式配置防止镜像、标签和子模块被一并推送。
	if _, e = g.Run(ctx, path, "-c", "remote."+target.Remote+".mirror=false", "-c", "push.followTags=false", "push", "--porcelain", "--no-force", "--no-mirror", "--no-follow-tags", "--recurse-submodules=no", "--", target.Remote, target.LocalOID+":"+target.RemoteRef); e != nil {
		return fail(e)
	}
	phase = model.Verify
	remote, e = RemoteHead(ctx, g, path, target.Remote, target.RemoteRef)
	if e != nil {
		return fail(e)
	}
	if remote != target.LocalOID {
		return fail(fmt.Errorf("推送命令已返回，但远程提交与预期不一致"))
	}
	return model.Published(target)
}
