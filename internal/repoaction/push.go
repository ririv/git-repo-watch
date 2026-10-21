package repoaction

import (
	"context"
	"fmt"
	"strconv"
	"strings"

	"gw/internal/gitcmd"
	"gw/internal/model"
	"gw/internal/publish"
)

type PushPlan interface {
	pushPlan()
	Path() string
	Target() model.PushTarget
	URL() string
}
type pushPlan struct {
	path   string
	target model.PushTarget
	url    string
}

func (*pushPlan) pushPlan()                  {}
func (p *pushPlan) Path() string             { return p.path }
func (p *pushPlan) Target() model.PushTarget { return p.target }
func (p *pushPlan) URL() string              { return p.url }

func PreparePush(ctx context.Context, g gitcmd.Runner, path string) (PushPlan, error) {
	if e := gitcmd.RequireIdle(ctx, g, path); e != nil {
		return nil, e
	}
	head, e := gitcmd.ReadHead(ctx, g, path)
	if e != nil {
		return nil, e
	}
	existing, ok := head.(gitcmd.ExistingHead)
	if !ok {
		return nil, fmt.Errorf("当前分支还没有提交")
	}
	ref := "refs/heads/" + existing.Ref
	text := func(args ...string) (string, error) { return gitcmd.Text(ctx, g, path, args...) }
	row, e := text("for-each-ref", "--format=%(upstream)%09%(upstream:remotename)%09%(upstream:remoteref)", ref)
	if e != nil {
		return nil, e
	}
	fields := strings.Split(row, "\t")
	if len(fields) != 3 {
		return nil, fmt.Errorf("无法读取当前分支的远程上游")
	}
	upstream, remote, remoteRef := fields[0], fields[1], fields[2]
	if upstream == "" || remote == "" || remote == "." {
		return nil, fmt.Errorf("当前分支没有配置远程上游，请先显式配置跟踪关系")
	}
	url, e := text("remote", "get-url", "--all", remote)
	if e != nil {
		return nil, e
	}
	pushURL, e := text("remote", "get-url", "--push", "--all", remote)
	if e != nil {
		return nil, e
	}
	if strings.Contains(url, "\n") || url != pushURL {
		return nil, fmt.Errorf("推送地址与获取地址不同，或存在多个地址；请使用 Git 显式处理")
	}
	if _, e = g.Run(ctx, path, "fetch", "--no-recurse-submodules", "--", remote); e != nil {
		return nil, e
	}
	remoteOID, e := publish.RemoteHead(ctx, g, path, remote, remoteRef)
	if e != nil {
		return nil, e
	}
	tracked, e := text("rev-parse", "--verify", upstream)
	if e != nil {
		return nil, e
	}
	if tracked != remoteOID {
		return nil, fmt.Errorf("远程上游引用不一致，请重新检查 fetch 配置或稍后重试")
	}
	counts, e := text("rev-list", "--left-right", "--count", existing.OID+"..."+remoteOID)
	if e != nil {
		return nil, e
	}
	pair := strings.Fields(counts)
	if len(pair) != 2 {
		return nil, fmt.Errorf("Git 提交数量格式错误")
	}
	ahead, e := strconv.Atoi(pair[0])
	if e != nil {
		return nil, e
	}
	behind, e := strconv.Atoi(pair[1])
	if e != nil {
		return nil, e
	}
	if behind > 0 {
		return nil, fmt.Errorf("当前分支领先 %d、落后 %d 个提交；请先手动处理远程更新", ahead, behind)
	}
	if ahead == 0 {
		return nil, fmt.Errorf("当前分支已与远程一致，没有待推送提交")
	}
	current, e := gitcmd.ReadHead(ctx, g, path)
	if e != nil {
		return nil, e
	}
	if current != head {
		return nil, fmt.Errorf("准备期间分支或 HEAD 已变化，请重新操作")
	}
	target := model.PushTarget{Branch: existing.Ref, LocalRef: ref, LocalOID: existing.OID, Remote: remote, RemoteRef: remoteRef, RemoteOID: remoteOID, Ahead: ahead}
	return &pushPlan{path, target, url}, nil
}

func Push(ctx context.Context, g gitcmd.Runner, review PushPlan) model.Push {
	p := review.(*pushPlan)
	head, e := gitcmd.ReadHead(ctx, g, p.path)
	if e != nil {
		return model.PublicationFailed(p.target, model.Validate, e.Error())
	}
	if current, ok := head.(gitcmd.ExistingHead); !ok || current.Ref != p.target.Branch || current.OID != p.target.LocalOID {
		return model.PublicationFailed(p.target, model.Validate, "分支或 HEAD 已变化，请重新确认推送")
	}
	url, e := gitcmd.Text(ctx, g, p.path, "remote", "get-url", "--all", p.target.Remote)
	if e != nil {
		return model.PublicationFailed(p.target, model.Validate, e.Error())
	}
	if url != p.url {
		return model.PublicationFailed(p.target, model.Validate, "远程地址已变化，请重新确认推送")
	}
	return publish.Forward(ctx, g, p.path, p.target)
}
